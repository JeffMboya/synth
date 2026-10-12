# Greenhouse irrigation controller (SYN-39)

Status: design, placement and copper. Routed clean by KiCadRoutingTools
9399bfb — 0 open nets, no via-in-pad, nothing below the board floor — but
`kicad-cli` DRC has not run on it here, so the verdict is `review_required`
and not `routed`. Not fabricated, not tested on hardware. Do not call it
proven. What is unchecked is listed at the bottom.

Produced from a prompt through the Synth MCP server. The prompt is the header
comment of `greenhouse_irrigation.synth`.

## What it is

A two-zone irrigation controller: it measures soil, decides, opens a valve,
and checks that water moved.

- 12 V in on a JST XH 2-pin (J1), the horticultural solenoid standard.
- Protection in series: F1 polyfuse, Q1 P-FET against reverse polarity,
  D1 zener clamping Q1's gate, D2 TVS array on the rail and the return.
- U2 MP2307 buck to 3.3 V. R2/R9 (10k / 3k01) against the 0.925 V reference.
- U3 STM32G030K8T6 in LQFP-32, 8 MHz crystal Y1, SWD on J3, reset on SW1.
- **Soil probes, two zones**: U7 TPS22918 gates their 3.3 V excitation from
  PB0; J6 and J7 carry excitation, return and ground. Each return is filtered
  (10k / 100nF) into PA1 and PA2.
- **Flow meter** on J8: 3.3 V, open-collector pulse into PA3 pulled up by
  R16, debounced by C21.
- **Valves, two zones**: Q2 on PA4 through J4, Q3 on PA5 through J5, each
  with its own flyback diode (D3, D6) and gate network.
- Ambient sensing on one I2C bus: U4 HDC1080 (temp/humidity, 0x40), U5
  BH1750 (lux, 0x23), U6 24LC256 for schedule and calibration (0x50), plus
  J2 as a spare I2C port. R5/R6 (4k7) are the only pull-ups.
- D4 green on the 3V3 rail; D5 on PA6 means irrigating.
- Source: `greenhouse_irrigation.synth`. Sidecars:
  `greenhouse_irrigation.placement.layout.toml` (board mm),
  `greenhouse_irrigation.schematic.layout.toml` (sheet mm).

## Why the probe excitation is switched

This is the one circuit on the board specific to soil rather than to sensing
in general. A probe held at a constant DC bias electrolyses: the electrodes
corrode, and the reading drifts as they do. So the probes are not powered
from the rail. U7 gates their excitation, the MCU raises it for the
milliseconds a conversion takes, and the probe is unbiased the rest of the
cycle.

The filters are anti-alias, not decoupling: 10 kOhm with 100 nF rolls off at
about 160 Hz — far below the sample rate, and orders above the rate at which
soil moisture changes.

Both zones share one excitation gate. That is deliberate: they are read in
sequence, never together, so a second switch would buy nothing.

## Why the valves are switched on the low side

Each coil sits between the protected 12 V rail and its FET's drain, so only
the return is switched and the MCU never sees 12 V.

| Condition | Gate | Valves | Notes |
| --- | --- | --- | --- |
| MCU running, PA4/PA5 low | held low by R11/R13 | closed | |
| MCU running, pin high | driven through R10/R12 | that zone open | |
| MCU in reset or unpowered | held low by R11/R13 (100k) | closed | fail-safe: a dead controller does not water |
| 12 V present, 3V3 absent | held low by R11/R13 | closed | |

D3 and D6 are reversed across their coils — cathode to 12 V, anode to the
drain — so each conducts only when its field collapses. Without them that
transient goes into the FET's avalanche rating on every close.

R11 and R13 (100 kOhm) are what make the fail-safe row true, and are the two
resistors on this board that must not be left off.

## Why the flow meter is there

A controller that opens a valve on a timer is a timer. The flow meter is how
this one learns that water actually moved: no pulses with a valve open means
a dry line, a closed stopcock or a failed solenoid; a rate far above
expectation means a burst. Both are told apart in firmware by rate, not by
hardware.

## Why 12 V entry is protected in that order

Q1's source faces the incoming rail, so its body diode conducts forward-only
and a reversed supply leaves the gate at source potential — off. R1 pulls the
gate down to enrol it; D1 keeps V_GS inside the 20 V limit at the top of the
supply range. F1 is ahead of both, so a shorted Q1 does not turn the cable
into the fuse.

C16 is bulk at the valves rather than at the buck: solenoid switching current
is sourced beside the connectors instead of through the buck's input cap.

## Unused and strapped pins

- `U6.a0`, `a1`, `a2` to ground: address 0x50, clear of the HDC1080 (0x40)
  and the BH1750 (0x23).
- `U6.wp` to ground: calibration and schedule stay writable in the field.
- `U4.csb` to its own `vddio`: selects I2C rather than SPI.
- `U4.sdo` to ground: address 0x40 rather than 0x41.
- `U4.gnd2`: the Bosch part's second ground pad, tied.
- `Y1.case_1`, `case_2`: the crystal can, grounded for shielding.
- `U7.ct`: no soft-start capacitor; the probe load is a few milliamps and
  needs none. `U7.qod`: quick output discharge left open, since the filters
  discharge through their own resistors.
- The STM32's remaining GPIO is not brought out. On an LQFP-32 with this
  function set there is no header budget, and an unconnected GPIO is not a
  defect.

## BOM

| Ref | Part | Registry id | Note |
| --- | --- | --- | --- |
| U3 | STM32G030K8T6 | `stm32g030` | MPN corrected in this PR; the part named three different devices |
| U2 | MP2307 | `mp2307_buck` | no MPN in the registry |
| U7 | TPS22918 | `tps22918_loadswitch` | no MPN in the registry |
| U4 | HDC1080DMBR | `hdc1080_humidity` | |
| U5 | BH1750FVI | `bh1750_lux` | no MPN in the registry |
| U6 | 24LC256-I/SN | `24lc256_eeprom` | address straps verified on pads 1-3 |
| Y1 | 8 MHz, 3.2x2.5mm 4-pin | `xtal_8mhz` | description corrected in this PR |
| Q1 | Si2301 | `p_mosfet_si2301` | no MPN in the registry |
| Q2, Q3 | IRLML6244 | `n_mosfet_irlml6244` | no MPN in the registry |
| D2 | TVS array, SOT-363 | `tvs_array_5v` | description corrected in this PR |
| D3, D6 | 1N5819 | `d_schottky` | |
| J1, J4, J5 | JST XH 2-pin | `jst_xh_2pin` | power in, valve zone 1, valve zone 2 |
| J6, J7, J8 | JST PH 3-pin | `jst_ph_3pin` | probe Z1, probe Z2, flow meter |
| J2 | JST PH 4-pin | `jst_ph_4pin` | spare I2C port |
| J3 | 1x4 header | `header_1x4` | SWD |

Twenty of this board's parts carry no distributor identity, so this BOM
cannot be ordered as it stands. That is #51, not an omission here.

## Reproducibility

Both sidecars are pinned, and the placement one is a fixed point: the placer
reproduces its 64 coordinates exactly, and two runs from it agree on every
component. Getting there took one iteration, because edge legalisation runs
after a `hard` override and moved the fuse and the edge connectors;
re-pinning from that output converged. A `hard` placement override is a
strong hint for an edge-adjacent part, not a final word.

A via count is not a fingerprint: the board name seeds the deterministic
uuids, so renaming changes the order the router works in. Connectivity is
what to compare.

## What is not checked

- `kicad-cli pcb drc` has not run on the routed copper. The independent
  geometric checks pass; KiCad's own DRC is the one that has not.
- No Gerbers, no drill, no position file, no STEP. The export stops short of
  manufacturing artifacts while the board is not fabrication-ready.
- **Synth does not know the probe returns are analogue.** `stm32g030`
  declares no `analog_input` capability on any pin, so PA1 and PA2 are
  ordinary bidirectional GPIO as far as validation is concerned. The ADC path
  — reference, impedance, settling against the 10k source — is unverified by
  anything here and is a reviewer's job.
- No thermal analysis. The buck at 3 A and two FETs switching inductive loads
  all deserve one before fabrication.
- `E-SYNTH-POWER-004` still reports the 3V3 rail carrying more load than its
  decoupling count supports. The rail has 24 pads and no plane, because Synth
  pours ground only (#117); on this board it routes clean regardless.
- Twelve `E-SYNTH-SCHEM-003` decoupling-distance warnings are outstanding.
  They quote schematic separations that are not credible for a board this
  size and depend on KiCad symbol geometry absent from the machine that
  produced this, so read them from CI rather than trusting them here.
- `E-SYNTH-SCHEM-010` asks for an explicit rail name instead of `VDD`.
  SynthSpec V1 has no net-naming syntax, so it cannot be acted on.
- The 0.925 V reference and the 10k/3k01 divider come from the MP2307
  datasheet, not measurement. 3.3 V is nominal.
- The I2C pull-up value (4k7) is a convention for a four-node 3.3 V bus at
  100 kHz, not a computed result for this trace length and capacitance.
- Not fabricated. Nothing on this board has been powered.
