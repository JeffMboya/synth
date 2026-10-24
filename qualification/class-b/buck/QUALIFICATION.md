# Qualification matrix: 24 V to 3.3 V buck board (SYN-67)

**Status: requires expert review. This is not a completed qualification result.**
Nothing here was fabricated or powered. Section A lists what Synth or KiCad checked and the
evidence. Section B lists what no tool here proves and an SI/PI/RF or power expert has to sign
off. No expert has reviewed this board.

This is one board in Class B (#38) and it carries **case 1 only**, the switching regulator. Two of
that case's four named features are declared unsupported rather than skipped: power-good (B3) and
thermal copper (B2). The analog front end, the USB/Ethernet peripheral and the other negative
cases belong to other boards. Section C maps every `#38` checklist item to a row here, including
the ones that do not apply.

## Reproduce

Run from the repository root on Synth `main` at `1eac8a1` or later (it includes the filled-pour
delivery #114, the label placement #144 and the 1.5 mm edge rim #272). Use an empty
`XDG_DATA_HOME` so a user registry cannot shadow parts.

```
export XDG_DATA_HOME=$(mktemp -d)
scripts/setup-routing-engines.sh --freerouting-only
B=$(pwd)/qualification/class-b/buck
synth validate $B/buck.synth
synth validate $B/buck_neg_fb_sw.synth                     # must exit 1 with E-SYNTH-POWER-011
synth export-kicad $B/buck.synth --out out/buck \
  --allow-unverified-parts --validate-erc --router free-routing \
  --pnp --gerbers --drill                                  # the fab package, reports/release-fab.json
synth export-kicad $B/buck.synth --out out/buck-nofab \
  --allow-unverified-parts --validate-erc --router free-routing   # source/DRC gate, reports/release.json
kicad-cli pcb drc --severity-all --refill-zones --format json \
  -o $B/reports/buck.kicad-drc.json $B/kicad/buck.kicad_pcb
kicad-cli sch erc --severity-all --format json \
  -o $B/reports/buck.kicad-erc.json $B/kicad/buck.kicad_sch
kicad-cli sch export netlist --format kicadsexpr -o /tmp/buck.net $B/kicad/buck.kicad_sch
(cd $B && sha256sum -c SHA256SUMS)
```

Tool versions used: KiCad 10.0.7, FreeRouting 2.4.1 on OpenJDK 25.0.5, Synth 0.1.1 built from
`main` `1eac8a1`, on 2026-10-24. Every export run at this revision produced the same
`.kicad_pcb` (`eda864e3...`) and `.kicad_sch` (`19a2f151...`). A different placement or router
version can give different copper. `out/` is a scratch directory and is not committed, so the paths in
`reports/` do not resolve from a fresh checkout.

## Artifact hashes

`SHA256SUMS` lists every design artifact (source, sidecar, KiCad project, Gerbers, drill files,
BOM, pick-and-place, pictures, routing, DRC and ERC reports). It leaves out `README.md`,
`QUALIFICATION.md` and itself. The key ones:

| Artifact | SHA-256 |
| --- | --- |
| `buck.synth` | `f723600457999808dee340bcdc7b75850056e3df1d4cefd5c2686faf1cdf5796` |
| `buck.placement.layout.toml` | `26f162ecff18fd997af69431fc21b7c9a42fb53876860d2e137242c1c6710d64` |
| `buck_neg_fb_sw.synth` | `928ba7985c9722f259d56a6a2c491b5c4aad4dece47b0402bf1c79b408c60c06` |
| `kicad/buck.kicad_pcb` | `eda864e33ca155050d06e17bfe8f8744c66577f5c87dfb16499fdbbd0b173f9e` |
| `kicad/buck.kicad_sch` | `19a2f15154fa16cfee69204b3737eb93de92c66f745613a72107d81741e68c24` |
| `kicad/bom.csv` | `6bcdcd49cdd480d13f13a3ca4052ab3149f22614b13f9f347a6a9443c0b9e50d` |
| `reports/release.json` | `5b06193026279d6f0001bb824430e028179fb2725a7cfd45a16f38e19a44b22a` |
| `reports/release-fab.json` | `d7ac4a82cc2d5844039f7af3d2d3de7199c82e36dd9331abae504e4feced6447` |
| `reports/buck.kicad-drc.json` | `8e0b51052ce047ac45251c029e3835af420fa5e1ffaedc8d81c2916be0eac51b` |
| `reports/buck.kicad-erc.json` | `cca9200894ac6491f7d69ecb1d2a1619a87d19c77d14500f7989ba6bf2fb90c3` |

Datasheet values used for the part values are in `README.md` ("Parts and datasheets"). The
datasheets are not committed, so the figures taken from them cannot be checked from this
repository.

## A. Checked by tools

| # | Claim | Evidence | Result |
| --- | --- | --- | --- |
| A1 | The source parses, resolves and lowers | `synth validate buck.synth` | Exit 0, 0 errors, 14 warnings: 8 `W-SYNTH-PART-UNVERIFIED`, 3 `E-SYNTH-SCHEM-003`, 1 `E-SYNTH-SCHEM-005`, 1 `W-SYNTH-ANOMALY-001`, 1 `W-SYNTH-SUPPLY-002` |
| A2 | Regulator feedback is not taken from the switch node | `E-SYNTH-POWER-011` on `buck_neg_fb_sw.synth` | The twin exits 1 with the error and 13 warnings; the good board does not raise it. The rule follows one series resistor only, so a longer feedback path is not checked |
| A3 | Every net is connected end to end | `reports/buck.connectivity.json`, `reports/buck.routing.json` | 10 nets, 0 open, 0 unconnected pads, 51 segments, 15 vias, 155.16 mm of copper, no via-in-pad |
| A4 | No KiCad DRC rule violation on the delivered board | `kicad-cli pcb drc --refill-zones`, `reports/buck.kicad-drc.json` | 0 errors, 0 unconnected, 20 warnings, all `lib_footprint_issues` (library links). No silkscreen warning |
| A5 | The route passes Synth's source/DRC gate | `reports/release.json` (`synth export-kicad`, no fab flags) | `release_ready` true, `production_status` production, state `routed`, 0 blocking signal-integrity findings |
| A6 | The package is not a fabrication release | `reports/release-fab.json` (`--gerbers --drill`) | `release_ready` **false**, `production_status` `untrusted`, override `unverified_parts` naming 7 parts. This is the correct answer until #51 lands a part review |
| A7 | Schematic electrical rules | `kicad-cli sch erc`, `reports/buck.kicad-erc.json` | 0 errors, 54 warnings: 34 `lib_symbol_issues`, 20 `footprint_link_issues` (the KiCad libraries are not in the config) |
| A8 | The schematic netlist matches the source | `kicad-cli sch export netlist` | 10 nets, node for node the source's 5 `net` blocks and 5 `connect` statements: GND 12 nodes, `+3V3` 7, VIN 7, FB 4, SW 3, EN 3, BOOT 2, VIN_FUSED 2, VIN_RAW 2, LED_PWR 2 |
| A9 | A layer change without a nearby ground return via is reported | `W-SYNTH-DRC-016` in both release files | One advisory finding: net 3 changes layers at (35.97, 14.43) mm with no ground via within 0.64 mm. The via at that point is on `+3V3`; "net 3" is Synth's numbering, and in the `.kicad_pcb` net 3 is FB, which stays on F.Cu. Not blocking; whether it matters is for the reviewer. `synth capability list` still declares return-path analysis unsupported, so read this check as a proximity test, not a return-current analysis |
| A10 | Manufacturing exports exist and are complete | `kicad/gerbers/` (22 Gerbers plus the `.gbrjob`), `kicad/drill/` | 4 copper layers (F, In1, In2, B), mask, paste, silkscreen and Edge.Cuts present, Edge.Cuts traces the 58.0 x 32.6 mm outline; PTH drill 19 holes (15 x 0.3 mm vias, 4 x 1.0 mm connector pins), NPTH none. Not fabricated |
| A11 | The exported project is readable by KiCad | KiCad 10.0.7 headless | `pcb drc`, `sch erc`, `sch export netlist`, `sch export pdf`, `pcb render` and `pcb export pos` all load the committed files and succeed. Not opened in the KiCad GUI |
| A12 | Ground pours are stored filled | `grep -c filled_polygon kicad/buck.kicad_pcb` | 4 filled polygons for the 4 GND zones, so KiCad opens showing the connectivity the gate validated |
| A13 | The placement sidecar is what was placed | sidecar compared with `kicad/buck.kicad_pcb` | All 20 parts match within 0.01 mm, except J1 and J2, which differ by the documented JST pin-1 anchor offset (-1.25, -0.525) mm |
| A14 | The placement passes Synth's structural review | MCP `synth_describe_placement` with the sidecar | `requires_revision` false, score 100, no findings, no functional warnings. The reported outline (63.0 x 37.6 mm) predates the 1.5 mm rim, so ignore that figure, not the verdict |

## B. Not proven: needs expert review

| # | Claim | Why it is not proven | Status |
| --- | --- | --- | --- |
| B1 | Survives a rated surge or a sustained 28 to 32 V rail | TPS54202 absolute maximum VIN is 30 V; SMBJ26A VBR is 28.9 to 31.9 V and clamps at 42.1 V (14.3 A). The TVS clamps surge energy only. A 24 V industrial rail needs a series OVP stage or a 40 to 60 V buck. Datasheet numbers only | Requires expert review |
| B2 | Thermal copper, thermal vias, thermal relief on pours and junction temperature | `synth capability list` declares `thermal_relief_analysis` unsupported: zone relief gap and spoke width are fixed export defaults and are not analysed, and there is no thermal-via or thermal-pour model. Estimated junction about 133 C at 70 C ambient (118.6 C/W) at 1 A, above the 125 C limit; about 100 C with the TI EVM figure 57.2 C/W. The 85% efficiency is an assumption | Unsupported by Synth; unverified by an expert |
| B3 | Power-good | The TPS54202 has no PG pin, and `synth capability list` declares `power_sequencing_and_power_good` unsupported: nothing declares or checks a power-good signal. Nothing in the design can carry one | Unsupported |
| B4 | Input and output filtering, loop stability, capacitor derating | Table 7-2 asks for 44 uF at 3.3 V; crossover 27.3 kHz at nominal value. DC-bias derating of the 10 uF input and 2 x 22 uF output ceramics is unknown (no vendor data). C1 electrolytic ESR unknown | Requires expert review |
| B5 | Switch-node loop area, return paths, EMI and ripple | Only pad-edge gaps were measured (README). No loop area, inductance, ripple or EMI result. FreeRouting ran with defaults, no length or coupling constraints. The `+3V3` sense track runs on B.Cu under the buck core | Requires expert review |
| B6 | Analog placement separation and grounding | `synth capability list` declares `split_ground_planes` unsupported: each ground net gets one unsplit zone. The FB divider ground is 4.25 mm from U1 pin 1 and reaches it through a via to the plane. Whether that is acceptable is for a reviewer | Requires expert review |
| B7 | Reverse-polarity block, PTC margin, UVLO thresholds, output voltage | All by hand calculation from datasheet numbers (README): PTC 0.248 A vs Ihold 0.63 A at 70 C with the assumed efficiency; UVLO 18.31 / 15.67 V; Vout 3.293 V. No bench measurement | Requires expert review |
| B8 | Part pins and footprints against the manufacturer drawings | `synth registry qualify --part <id>` (output in `reports/registry-qualify.txt`): all 7 new parts blocked by `E-SYNTH-QUAL-008` — 6 for having no reviewer, the electrolytic for carrying no provenance — and `mf_msmf110_33x` additionally reports `E-SYNTH-QUAL-016` (its footprint names a 3D model that is not installed). The stock KiCad `SOT-23-6` footprint was not compared with the TI DDC drawing. LED datasheet unreachable; connectors and generic passives have no MPN | Requires expert review |
| B9 | Compactness | Outline 58.0 x 32.605 mm (1891 mm2) with the 1.5 mm rim, down from 63.0 x 37.6 mm (2369 mm2) before #272. No auto-placed run was made at this revision, so the hand layout is not compared against the placer | Not a defect; not re-measured |
| B10 | Keepouts and shielding | The language has keepouts; none was declared under the switch node or the inductor in this design, and none was checked | Unsupported here |
| B11 | Power sequencing, soft-start and inrush | UVLO thresholds are calculated (B7). Soft-start time, inrush into C1 and the output capacitors, and PTC behaviour in a surge were not analysed | Unverified |
| B12 | Decoupling placement | C3 (100 nF) is 1.18 mm from the U1 VIN pad, C7 0.75 mm from R_FB_T, in the pad-gap table in `README.md` (`pad_gaps.py`). Loop inductance was not computed | Unverified |
| B13 | Stackup and impedance assumptions | The board uses the default 4-layer stackup with inner ground planes. No stackup block was declared, no controlled-impedance net exists, and the capability descriptor marks inner-layer impedance unsupported | Not applicable to a single-ended power board |
| B14 | What the Gerbers, drill files and BOM say about orderability | The files in `kicad/` were generated and hashed (A10), but nothing was fabricated, no fab accepted them, no part carries an LCSC number, and the electrolytic and every generic passive have no MPN either. Availability not checked | Not done |
| B15 | Schematic correctness and readability | ERC has 0 errors (A7) and the netlist matches the source (A8). The sheet was rendered and read: `tools/schematic_quality.py` reports 1 overlapping text pair, 3 texts with a wire through them, 5 wire crossings, 803.9 mm of wire, and a mean passive-to-pin distance of 83.8 mm (max 148.3). C5's `+3V3` wire passes the BOOT pin stub without a junction, so at print size it reads like a boot capacitor; the netlist is correct | Readable, not tidy |
| B16 | The other negative cases from #38 (missing pair constraints, invalid stackup assumptions) | This board has no differential pair and no stackup claim, so those cases do not apply here; the USB board (#103) carries them. Only the unsafe-feedback case is built here, and the rule follows one series resistor only | Not applicable here |
| B17 | Comparison with a human reference design | Only the TI EVM thermal figure (57.2 C/W) was used. No reference layout was compared (#68) | Not done |
| B18 | Function on real hardware | Nothing was powered, programmed or measured. There are no mounting holes, so the board as it stands cannot be fixed down | Not done |
| B19 | Independent mixed-signal / RF sign-off (#38) | No reviewer | Not done |

Rows A2, A9 and B1 to B8 are the ones that decide whether this board meets a Class B bar (#38
treats missing SI/PI/RF evidence as a failure). They are open.

## C. #38 traceability for case 1

Every line of `#38`'s "Check exactly" list, against this board.

| `#38` item | Verdict here | Where |
| --- | --- | --- |
| Differential-pair membership and geometry, length constraints | Not applicable: no differential pair on this board | B16 |
| Return paths and layer transitions | One advisory proximity finding; no return-current analysis, and the capability descriptor says so | A9 |
| Impedance assumptions | Not applicable: no controlled-impedance net, no stackup block | B13 |
| Keepouts | Unsupported here: the language has them, none was declared or checked | B10 |
| Analog placement separation | FB divider to U1 FB 1.98 mm, divider ground 4.25 mm from U1 GND through a via; not judged | B6 |
| Grounding | One unsplit GND zone per layer; split planes unsupported; see the layer-change finding | B6, A9 |
| Decoupling | Boot and decoupling capacitors are 0.75 to 4.34 mm from the pin they serve (C7 0.75, C3 1.18, C4 2.77, C2 4.34); loop inductance not computed | B12 |
| Thermal relief | Fixed export defaults, not analysed; no thermal-via model | B2 |
| Connector ESD paths | Only the input TVS (D2) on VIN; J1 and J2 are 2-pin power connectors with no signal pins, so no ESD array is claimed | B1, README |
| Power sequencing | Unsupported: the part has no power-good pin and nothing declares or checks sequencing; UVLO thresholds are by hand | B3, B7, B11 |
| ERC and DRC | 0 errors on both, 20 + 54 library-link warnings | A4, A7 |
| Schematic correctness | Netlist equals the source net for net; readability measured, not tidy | A8, B15 |
| BOM correctness | 20 components, one BOM row each, no MPN or LCSC on the generic parts | A6, B14 |
| Footprint correctness | Pins and packages not compared with the manufacturer drawings; none of the 7 new parts has a reviewer | B8 |
| Manufacturing exports | Gerbers, drill, BOM and pick-and-place generated and hashed; not fabricated | A10, B14 |
| KiCad reopenability | Loaded headlessly by KiCad 10.0.7 for every export used here; not opened in the GUI | A11 |
| Which claims Synth proves | Section A, and only section A | A1-A14 |
| Which claims need SI/PI/RF expert analysis | Section B | B1-B19 |
| Comparison against a human reference | Only the TI EVM thermal figure; no layout comparison | B17 |

Two of `#38`'s four named regulator features are carried by this board's evidence, feedback
(A2) and input/output filtering (A3, B4, B12); power-good (B3) and thermal copper (B2) are
declared unsupported rather than skipped, which is what #67's acceptance criteria ask for.

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
rationale = "Class B case 1: a 24 V to 3.3 V TPS54202 buck with input protection, UVLO and a \
switch-node-feedback negative twin. The board is its own case: the feedback-safety rule, the \
unreviewed-part fab block and the layer-change finding are measured on this source, not carried \
over from another board."
```

`expect` and `expect_fab` are deliberately missing. `synth check` in this revision always
selects KiCadRoutingTools and has no flag to select FreeRouting, and no KiCadRoutingTools
checkout was installed here, so the `synth check` verdict for this board was not measured. The
measured facts that would feed it are `validate` exit 0 (A1), `routing.state` routed (A3) and
`kicad-cli pcb drc` 0 errors (A4) for source/DRC, and `release_ready` false with
`production_status` untrusted (A6) for manufacturing.
