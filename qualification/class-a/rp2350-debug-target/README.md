# RP2350 debug target

A minimal RP2350 board: USB-C power into a 3V3 LDO, the MCU's own crystal, a
tactile reset on RUN, a 3-pin SWD header, QSPI boot flash and one status LED.
No USB data path — that belongs to a faster class.

Design rationale is in the source comments next to the circuits it explains;
this file is the specification.

## Board

| | |
| --- | --- |
| Size | 77.90 × 60.46 mm |
| Layers | 4 |
| Process | JLCPCB filled-via (0.127 mm track/clearance, 0.3 mm drill, 0.13 mm ring, `requires via_in_pad`) |
| Components | 25 |
| Nets | 20 |
| Source | `rp2350_debug_target.synth` |

## Why four layers

The RP2350's QFN-60 is a 0.4 mm pitch package leaving 0.200 mm between pads,
and a via on this process needs 0.687 mm between centres. The drill that would
fit between two pins is 0.013 mm, which no fabricator offers. Synth's escape
analysis and KiCadRoutingTools reach that conclusion independently.

Two layers cannot route this part at all. Four can, and the price is the
filled-via tier: 19 vias still land in pads, because on a 0.4 mm pitch there
is nowhere else for them to go. Four layers buy the escape; declaring
`requires via_in_pad` buys the right to plate it.

## Connectors

| Ref | Part | Carries |
| --- | --- | --- |
| J1 | USB-C receptacle | 5 V in, CC resistors to 5.1k |
| J2 | 1×3 header, 2.54 mm | SWCLK, SWDIO, GND |

## Measured

One local run of the export and the external router. No figure here came from
anywhere else.

| | |
| --- | --- |
| Nets closed | **20 of 20** |
| Open | **0** |
| Routed copper | 624.6 mm over 209 segments and 47 vias |
| Tracks below the 0.127 mm floor | **0** |
| Vias below the 0.3 mm drill or 0.13 mm ring | **0** |
| Vias inside pads | 19, all on the declared filled-via process |
| `kicad-cli pcb drc` | **0 errors**, 0 unconnected items, 2 warnings |
| Routing verdict | `routed`, fabrication-ready |

### DRC detail

| Severity | Finding | Count |
| --- | --- | --- |
| warning | via has unconnected end | 2 |

Two things had to be said out loud before this board closed. The crystal and
the reset parts are hinted onto `U2`, because XIN/XOUT and RUN are the two
nets here whose length is the circuit rather than just the routing — without
that, `Y1` landed wherever the placer had room and XOUT stayed open. And the
process is `jlc-filled-via` with `requires via_in_pad`, because 19 vias land
in pads on a 0.4 mm pitch package and there is no 0.2 mm gap to put a 0.56 mm
via in.

The router's own policy rejects those 19 vias; Synth's gate accepts them
because the board declares the process that plates them. That is the
difference between a via in a pad by accident and one in a pad on purpose.

The twenty-two `annular_width` errors of the 16 October run are gone with
#216, and nothing on this board is below a fabrication floor.

## Limitations

- **This board is laid out to the wrong device.** `rp2350` carries RP2040 pin
  numbers (#152), so every net on the QFN-60 reaches the pad an RP2040 would
  have. Nothing below matters until that part is corrected, and no routing
  result from this board should be read as a Class-A pass.
- **The filled-via process is a cost, not a free pass.** 19 of the 47 vias
  land in pads, so this board cannot be built on the standard tier at all.
  That is the QFN-60's geometry and it is why the tier is declared rather
  than assumed, but a reviewer should agree the part is worth it before the
  board is ordered.
- **Two vias have an unconnected end.** Advisory: every net is closed and no
  pad is unconnected, so they are redundant copper.
- **Four `E-SYNTH-ESD-001` findings.** The USB-C rail has no
  reverse-polarity protection and neither SWD signal has an ESD clamp, which
  on a board whose whole purpose is to be plugged into other people's
  equipment is a real omission rather than a cosmetic one.
- Four of its parts carry no distributor identity, so the BOM is not orderable
  as it stands. No independent reviewer has signed off.
