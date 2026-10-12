# Greenhouse irrigation controller (SYN-39)

Status: design, placement and copper. Routed clean by KiCadRoutingTools
9399bfb — 24 nets, no open net, no via-in-pad, nothing below the board floor —
but `kicad-cli` DRC has not run on it here, so the verdict is
`review_required` and not `routed`. Not fabricated, not tested on hardware.
Do not call it proven. What is unchecked is listed at the bottom.

Produced from a prompt through the Synth MCP server. The prompt is the header
comment of `greenhouse_irrigation.synth`.

## What it is

- 12 V in on a JST XH 2-pin (J1), the horticultural solenoid standard.
- Protection in series: F1 polyfuse, Q1 P-FET against reverse polarity,
  D1 zener clamping Q1's gate, D2 TVS array on the rail and the return.
- U2 MP2307 buck to 3.3 V. R2/R9 (10k / 3k01) set the divider against the
  0.925 V reference. L1 is the 10 µH output inductor.
- U3 STM32G030K8T6 in LQFP-32, 8 MHz crystal Y1 with C9/C10 load caps,
  SWD on J3 (3V3, SWDIO, SWCLK, GND), reset on SW1.
- One I²C bus: U4 HDC1080 (air temp/humidity, 0x40), U5 BH1750 (lux, 0x23),
  U6 24LC256 EEPROM (calibration, 0x50), and J2 for an external soil probe.
  R5/R6 (4k7) are the only pull-ups on the bus.
- Q2 IRLML6244 switches the solenoid return. J4 carries the coil.
- D4 green on the 3V3 rail, D5 red on PA5 beside the valve gate on PA4.
- Source: `greenhouse_irrigation.synth`. Sidecars:
  `greenhouse_irrigation.placement.layout.toml` (board mm),
  `greenhouse_irrigation.schematic.layout.toml` (sheet mm).

## Why the valve is switched on the low side

The coil sits between the protected 12 V rail and Q2's drain, so only the
return is switched and the MCU never sees 12 V.

| Condition | Gate | Valve | Notes |
| --- | --- | --- | --- |
| MCU running, PA4 low | held low by R11 | closed | |
| MCU running, PA4 high | driven through R10 | open | |
| MCU in reset or unpowered | held low by R11 (100k) | closed | fail-safe: a dead controller does not water |
| 12 V present, 3V3 absent | held low by R11 | closed | |

D3 is reversed across the coil — cathode to 12 V, anode to Q2's drain — so it
conducts only when the field collapses and drives the drain above the rail.
Without it that transient goes into Q2's avalanche rating on every close.

R10 (100 Ω) limits the gate drive; R11 (100 kΩ) is what makes the fail-safe
row above true, and is the one resistor on this board that must not be left off.

## Why 12 V entry is protected in that order

Q1's source faces the incoming rail, so its body diode conducts forward-only
and a reversed supply leaves the gate at source potential — off. R1 pulls the
gate down to enrol it; D1 keeps V_GS inside the 20 V limit when the supply is
at the top of its range. F1 is ahead of both, so a shorted Q1 does not turn
the cable into the fuse.

C16 is bulk at J4 rather than at the buck: the solenoid's switching current is
sourced beside the connector instead of being pulled through the buck's input
capacitor.

## Unused and strapped pins

- `U6.a0`, `a1`, `a2` to ground: address 0x50, clear of the HDC1080 (0x40) and
  the BH1750 (0x23).
- `U6.wp` to ground: calibration stays writable in the field.
- `U4.csb` to its own `vddio`: selects I²C rather than SPI.
- `U4.sdo` to ground: address 0x40 rather than 0x41.
- `U4.gnd_2`: the Bosch part's second ground pad, tied.
- `Y1.case_1`, `case_2`: the crystal can, grounded for shielding.
- The STM32's `pa0`–`pa3`, `pa6`–`pa15` except `pa13`/`pa14`, `pb0`–`pb5`,
  `pb8`, `pb9`, `pc14`/`pc15` beyond the crystal: not brought out. On an
  LQFP-32 with this function set there is no header budget for them, and an
  unconnected GPIO on an MCU is not a defect.

## BOM

| Ref | Part | Registry id | Note |
| --- | --- | --- | --- |
| U3 | STM32G030K8T6 | `stm32g030` | MPN corrected in this PR; the part named three different devices |
| U2 | MP2307 | `mp2307_buck` | no MPN in the registry |
| U4 | HDC1080DMBR | `hdc1080_humidity` | |
| U5 | BH1750FVI | `bh1750_lux` | no MPN in the registry |
| U6 | 24LC256-I/SN | `24lc256_eeprom` | address straps verified on pads 1-3 |
| Y1 | 8 MHz, 3.2x2.5mm 4-pin | `xtal_8mhz` | description corrected in this PR |
| Q1 | Si2301 | `p_mosfet_si2301` | no MPN in the registry |
| Q2 | IRLML6244 | `n_mosfet_irlml6244` | no MPN in the registry |
| D2 | TVS array, SOT-363 | `tvs_array_5v` | description corrected in this PR |
| D3 | 1N5819 | `d_schottky` | |
| J1, J4 | JST XH 2-pin | `jst_xh_2pin` | no MPN in the registry |
| J2 | JST PH 4-pin | `jst_ph_4pin` | no MPN in the registry |
| J3 | 1x4 header | `header_1x4` | no MPN in the registry |

Fourteen of this board's parts carry no distributor identity at all, so this
BOM cannot be ordered as it stands. That is #51, not an omission here.

## What is not checked

- `kicad-cli pcb drc` has not run on the routed copper. The independent
  geometric checks pass; KiCad's own DRC is the one that has not.
- No Gerbers, no drill, no position file, no STEP. The export stops short of
  manufacturing artifacts while the board is not fabrication-ready.
- No impedance claim is made and none is needed: there is no controlled
  impedance net on this board.
- The 0.925 V reference and the 10k/3k01 divider are taken from the MP2307
  datasheet, not measured. 3.3 V is nominal.
- The I²C pull-up value (4k7) is a convention for a four-node 3.3 V bus at
  100 kHz, not a computed result for this trace length and capacitance.
- No thermal analysis. The buck at 3 A and the FET switching an inductive
  load both deserve one before fabrication.
- Nine `E-SYNTH-SCHEM-003` decoupling-distance warnings are outstanding. They
  quote schematic separations that are not credible for a board this size and
  depend on KiCad symbol geometry absent from the machine that produced this,
  so they are to be read from CI rather than trusted here.
- `E-SYNTH-SCHEM-010` asks for an explicit rail name instead of `VDD`.
  SynthSpec V1 has no net-naming syntax, so it cannot be acted on.
- Not fabricated. Nothing on this board has been powered.

## Reproducibility

Both sidecars are pinned, and the placement one is a fixed point: the placer
reproduces its 47 coordinates exactly, and two runs from it agree on every
component. Getting there took one iteration, because edge legalisation runs
after a `hard` override and moved F1 and the four connectors by 1 to 4 mm;
re-pinning from that output converged. So a `hard` placement override is a
strong hint for edge-adjacent parts rather than a final word, which is worth
knowing before trusting one.

Routed from the pinned placement with KiCadRoutingTools 9399bfb: 24 nets, no
open net, no via-in-pad, nothing below the board floor, 57 vias.

The via count is not stable across a rename: the board name seeds the
deterministic uuids, which changes the order the router works in. The
placement is pinned and identical, and connectivity is clean either way, but
do not read a via count as a fingerprint.
