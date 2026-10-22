# RP2350 debug target

A minimal RP2350 board: USB-C power into a 3V3 LDO, the MCU's own crystal, a
tactile reset on RUN, a 3-pin SWD header, QSPI boot flash and one status LED.
No USB data path — that belongs to a faster class.

Design rationale is in the source comments next to the circuits it explains;
this file is the specification.

## Board

| | |
| --- | --- |
| Size | 77.90 × 62.95 mm |
| Layers | 4 |
| Process | JLCPCB standard (0.127 mm track/clearance, 0.3 mm drill, 0.13 mm ring) |
| Components | 25 |
| Nets | 20 |
| Source | `rp2350_debug_target.synth` |

## Why four layers

The RP2350's QFN-60 is a 0.4 mm pitch package leaving 0.200 mm between pads,
and a via on this process needs 0.687 mm between centres. The drill that would
fit between two pins is 0.013 mm, which no fabricator offers. Synth's escape
analysis and KiCadRoutingTools reach that conclusion independently.

Two layers cannot route this part. Four can, and on four the escape no longer
needs a via inside a pad — it has inner layers to change into. What four
layers do not buy is connectivity: two nets still fail to close, which is in
*Measured* below.

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
| Nets closed | 18 of 20 |
| Open | 2 — `net_8` at `U2.22` reaching 1 of its 3 pads, `net_9` at `C11.1` reaching 2 of its 5 |
| Routed copper | 624.6 mm over 293 segments and 42 vias |
| Tracks below the 0.127 mm floor | **0** |
| Vias below the 0.3 mm drill or 0.13 mm ring | **0** |
| Vias inside pads | **0** |
| `kicad-cli pcb drc` | **0 errors**, 1 unconnected item, 2 warnings |
| Routing verdict | `validation_failed` — not fabrication-ready |

### DRC detail

| Severity | Finding | Count |
| --- | --- | --- |
| error | missing connection between items | 1 |
| warning | track has unconnected end | 2 |

The twenty-two `annular_width` errors this board reported on 16 October are
gone, and so are the two vias that used to land inside pads: the escape now
sizes its vias from the same numbers the exported project states, so there is
nothing left for KiCad to disagree with. What remains is connectivity, which
is the harder half.

## Limitations

- **This board is laid out to the wrong device.** `rp2350` carries RP2040 pin
  numbers (#152), so every net on the QFN-60 reaches the pad an RP2040 would
  have. Nothing below matters until that part is corrected, and no routing
  result from this board should be read as a Class-A pass.
- **Two nets do not close.** `net_8` reaches 1 of its 3 pads at `U2.22`, and
  `net_9` — the RUN node, pulled up through R3, shorted by SW1 and debounced
  by C11 — reaches 2 of its 5. The router names them as the pair to route
  first on a retry.
- No fabrication artifacts are produced; the package is refused as
  `UNTRUSTED / NOT FOR FABRICATION`, and that refusal is correct while two
  nets are open.
- **Four `E-SYNTH-ESD-001` findings.** The USB-C rail has no
  reverse-polarity protection and neither SWD signal has an ESD clamp, which
  on a board whose whole purpose is to be plugged into other people's
  equipment is a real omission rather than a cosmetic one.
- Four of its parts carry no distributor identity, so the BOM is not orderable
  as it stands. No independent reviewer has signed off.
