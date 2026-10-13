# iCE40 HDMI video board — SYN-40

A Lattice iCE40 HX4K in a 121-ball BGA driving an HDMI Type-A output, with its
configuration flash, a 25 MHz oscillator, and 1.2 V and 3.3 V rails carried as
inner-layer planes from a single 5 V USB-C input.

**Status: not fabricated, not powered, not fully routed.** KiCadRoutingTools
caad45c closes 31 nets and leaves 10 open; `kicad-cli` DRC reports 63 errors and
13 unconnected pads. Both supply rails route as planes. One of the eight TMDS
lanes carries copper end to end. See *Limitations*.

Design rationale is in the source comments next to the circuits it explains;
this file is the specification.

## Board

| | |
| --- | --- |
| Size | 88.61 × 62.48 mm |
| Layers | 4 |
| Process | JLCPCB standard (0.127 mm track/clearance, 0.3 mm drill) |
| Components | 49 |
| Source | `ice40_hdmi_video.synth` |
| Sidecars | `ice40_hdmi_video.placement.layout.toml` (all 49 pinned) |

## Stackup

| Layer | Thickness | Carries |
| --- | --- | --- |
| F.Cu | 0.035 mm | signal, ground pour |
| prepreg | 0.2104 mm, εr 4.4 | — |
| In1.Cu | 0.0152 mm | 1.2 V core plane |
| core | 1.065 mm, εr 4.6 | — |
| In2.Cu | 0.0152 mm | 3.3 V I/O plane |
| prepreg | 0.2104 mm, εr 4.4 | — |
| B.Cu | 0.035 mm | signal, ground pour |

Total 1.586 mm. The 0.2104 mm prepreg under the signal layer is what puts a
0.2 mm trace near 50 Ω single-ended over the plane below it, which is the 100 Ω
differential the TMDS pairs want.

## Supply

| Rail | Source | Nominal | Carried as | Feeds |
| --- | --- | --- | --- | --- |
| 5 V in | J1 USB-C, CC1/CC2 pulled down 5.1 kΩ | 5 V | traces | U2 and U3 inputs, J2 pin 18 |
| 1V2 core | U2 MP2307 buck, L1 4u7, 3k01/10k divider | 1.2 V | **In1.Cu plane** | U1 VCC ×4, VCCPLL0/1 |
| 3V3 I/O | U3 AMS1117-3.3 | 3.3 V | **In2.Cu plane** | U1 VCCIO ×5, VCC_SPI, VPP_2V5, VPP_FAST, U4, Y1, pull-ups |

Each rail pad reaches its plane through a via in the pad. On this process that
is forced, not chosen — see *Escape arithmetic*.

## Connectors

All pinouts below are derived from the source, not transcribed.

**J1 — USB-C receptacle, power only**

| Pin | Signal |
| --- | --- |
| VBUS ×4 (A4/B4/A9/B9) | +5 V in |
| GND ×4 (A1/B1/A12/B12), SHLD | Ground |
| CC1, CC2 | 5.1 kΩ to GND, advertising a 5 V sink |
| SBU1, SBU2 | To D1 TVS channels 3 and 4 |

D+/D− are unused: this is a power-only inlet.

**J2 — HDMI Type-A receptacle**

| Pin | Signal |
| --- | --- |
| 1, 3 | TMDS data 2 ± (via R8/R9, 270 Ω) |
| 4, 6 | TMDS data 1 ± (via R10/R11) |
| 7, 9 | TMDS data 0 ± (via R12/R13) |
| 10, 12 | TMDS clock ± (via R14/R15) |
| 2, 5, 8, 11 | Lane shields, to ground |
| 15, 16 | DDC SCL/SDA, pulled up 47 kΩ to 3V3 |
| 17 | Ground |
| 18 | +5 V to the sink, decoupled by C16 |
| 19 | Hot-plug detect, to U1 ball H7 |

## FPGA ball map

ICE40HX4K-BG121, 121 balls on an 0.8 mm grid. Seventeen signal balls are used.

| Ball | Signal |
| --- | --- |
| A1, A2 | TMDS data 2 +/− |
| A3, A4 | TMDS data 1 +/− |
| A5, A6 | TMDS data 0 +/− |
| A7, A8 | TMDS clock +/− |
| B6 | 25 MHz global clock in, from Y1 |
| L10, K10 | Flash SCK, CS |
| J9, K9 | Flash SDI, SDO |
| L8, H9 | DDC SCL, SDA |
| H7 | HDMI hot-plug detect |
| L9 | CRESET, pulled up 10 kΩ |
| K8 | CDONE, pulled up 10 kΩ, drives D2 |

The four TMDS pairs sit on the top row of the grid, which is the one row that
escapes without a dog-bone.

Power and ground: 1.2 V on D4, D8, H4, H8 and PLL supplies J6, C6; 3.3 V on
VCCIO D6, F8, H6, E4, G4, VCC_SPI L11, VPP_2V5 C10, VPP_FAST B10; ground on ten
balls plus both PLL grounds.

## Netclasses

| Class | Track | Clearance | Other | Applies to |
| --- | --- | --- | --- | --- |
| TMDS | 0.2 mm | 0.2 mm | `match_length 0.25mm`, via 0.56/0.3 mm | the 8 lane nets |
| CORE | 0.6 mm | 0.25 mm | `plane` | the 1.2 V net |
| IO | 0.4 mm | 0.2 mm | `plane` | the 3.3 V net |

A `netclass` block assigns no nets. Each of these is joined by `class "NAME"` on
a `connect`, which is the spelling the language reference did not document until
#129. Without it a class is inert and reads as satisfied.

## Differential pairs

Four pairs, each `impedance 100ohm`, `max_skew 0.1mm`, `couple tight`, declared
on the connector side of the 270 Ω series resistors where the controlled run is.

`max_skew` is checked, not merely recorded: `E-SYNTH-DRC-014` sums each half of
the pair from the emitted segments and fails one whose halves differ by more
than the budget, whatever the router reported about its own matching. `couple`
is not checked — nothing keeps the two halves of a pair together.

The iCE40 has no dedicated TMDS transmitter, so the pairs are driven as
pseudo-differential LVCMOS33 from one bank with series termination, which is how
the open-source iCE40 HDMI designs do it. That is good for 640×480 and not for
1080p; a real 1080p board uses a transmitter such as the TFP410, and the
registry has no such part.

## Escape arithmetic

A property of the process, not of the router. At JLCPCB's standard floor a via
is 0.3 mm drill plus 2 × 0.13 mm annular, so 0.56 mm across. Dropped between two
balls 0.8 mm apart it sits 0.400 mm from each ball centre and needs 0.582 mm to
clear the pad copper. Dog-bone escape is therefore impossible on this tier, and
via-in-pad is the required technique rather than a shortcut.

That 0.56 mm via is now declared rather than assumed, and finding out why the
declaration had no effect turned up something worse. Until `via_diameter`
existed the exporter hardcoded via geometry twice: 0.6/0.3 into the board's
`net_class` block, which KiCad 7 and later ignore, and 0.6/**0.4** into the
project file, which is what KiCad and the router actually read. A 0.4 mm drill
inside a 0.6 mm pad is a 0.10 mm annular ring against this tier's 0.13 mm
floor, so every net class Synth had ever written asked for a via the declared
process would not plate. The measured run bears it out: 91 of the control
board's vias came back at 0.6/0.4 before the fix and none after it, with
connectivity unchanged.

The TMDS class now names the tier's smallest legal via, and
`E-SYNTH-PROCESS-003` checks it against the profile's drill and annular floors
instead of trusting it.

The board therefore declares `requires via_in_pad` and names a tier that offers
it. That is not a waiver. The requirement is checked against the process: the
same board against `manufacturer "jlcpcb"` is `E-SYNTH-PROCESS-002` rather than
a board that quietly routes, because a process that cannot fill and cap
via-in-pad cannot build it. The two tiers carry identical geometric floors —
filling a via buys the capability, not a finer drill — so nothing about the
escape geometry changes with the declaration.

What changes with the declaration is what the router is told, and it was
measured rather than assumed. Both runs below used KiCad 10.0.6,
KiCadRoutingTools caad45c with `grid_router` 0.23.0, the same pinned placement,
and sources differing only in the tier name, the `requires` line and the via
attributes:

| | nets closed | open | blocking | of those, via-in-pad | DRC err / unconn / warn |
| --- | --- | --- | --- | --- | --- |
| without the declaration | 30 | 11 | 52 | 39 | 63 / 14 / 40 |
| with it | 31 | 10 | 11 | 0 | 63 / 13 / 32 |

The declaration closes one more net, clears all 39 via-in-pad blockers, and
drops an unconnected pad and eight warnings. It costs nothing connected: one
TMDS lane reaches both ends either way. What the seven unrouted lanes lose is
copper that never connected them.

Nothing in CI re-measures this. The release routability gate covers
`sensor_logger`, `env_logger` and `secure_tracker`, and the qualification
matrix that would cover this board is not on this branch (#45). These figures
come from a local run; `scripts/setup-routing-engines.sh` sets up the engines,
and the router needs a Python that can import KiCad's `pcbnew`.

The declared 0.56 mm via has not been observed changing routed copper. The
class that declares it is the one that barely routes, so there are no TMDS vias
to size, and the 110 vias placed are the 0.6 mm default. The declaration is
correct and not yet load-bearing.

## Notable parts

| Ref | Part | Registry id |
| --- | --- | --- |
| U1 | ICE40HX4K-BG121 | `ice40hx4k_bg121` |
| U2 | MP2307 | `mp2307_buck` |
| U3 | AMS1117-3.3 | `ams1117_3v3` |
| U4 | W25Q128JV | `w25q128_flash` |
| Y1 | 25 MHz oscillator | `osc_smd_25mhz` |
| J1 | USB-C receptacle | `usb_c_receptacle` |
| J2 | HDMI Type-A receptacle | `hdmi_a_receptacle` |
| D1 | TVS array SOT-363 | `tvs_array_5v` |

`ice40hx4k_bg121` is the registry's first BGA part and `hdmi_a_receptacle` its
first HDMI connector; both were written for this board and verified ball by ball
against the stock KiCad symbol and footprint. The generated `bom.csv` is the
bill of materials; this table is the actives and omits 39 generic passives.

## Limitations

- **The board does not route completely.** 31 nets closed, 10 open: the 5 V
  input, four HDMI connector pins, and two-pad traces around the series
  resistors. That is signal routing a human would finish by hand, and #40 asks
  for exactly that list.
- **Seven of the eight TMDS lanes do not route.** Only `net_23` reaches both
  ends. The other seven are open in every run measured, before and after the
  via-in-pad declaration, so the video bus is the board's largest single gap.
  Nothing escapes the BGA on purpose yet: KiCadRoutingTools ships a fanout
  engine with an under-pad method for arrays this dense, and Synth does not
  invoke it (#137).
- **Length matching cannot be judged yet, and the check is silent rather than
  satisfied.** The class declares `match_length 0.25mm`. Before the via-in-pad
  declaration the seven unrouted lanes carried non-connecting stubs, and the
  check measured those: a 19.083 mm spread from `net_25` at 0.925 mm to
  `net_23` at 20.008 mm — a complaint about stub lengths, not lane lengths.
  With the declaration the stubs are gone, one lane carries copper, and a class
  with one measurable member is skipped: a net cannot be mismatched against
  itself. Read the silence as "not yet measurable", never as "matched".
- `kicad-cli pcb drc` reports 65 errors and 14 unconnected pads on the routed
  copper. No fabrication artifacts are produced.
- **Impedance is not verified on the inner layers.** Only outer-layer microstrip
  is derived from the stackup, so the four `W-SYNTH-IMPEDANCE-001` warnings say
  the pairs may be routed where impedance is not checked. A pair cannot be
  pinned to a layer at all (#118).
- Return paths, layer transitions, via stubs, neck-downs and plane continuity
  are not analysed. The compiler's capability descriptor lists all of them as
  unsupported.
- No thermal analysis and no power sequencing: enable ordering and power-good
  are neither declared nor checked.
- `E-SYNTH-POWER-004` reports the 5 V net carrying 11 load endpoints against 2
  decoupling capacitors. Fair on a board with this many supplies.
- Nine `E-SYNTH-SCHEM-003` warnings quote schematic separations that depend on
  KiCad symbol geometry; read them from CI, not from a local run.
- Four parts carry no distributor identity, so the BOM is not orderable as it
  stands (#51).

## What #40 asks, and what this board answers

| Case | Answer |
| --- | --- |
| BGA / fine-pitch escape, via and fanout rules | **Partly.** 121 balls on 0.8 mm. The escape requirement and the escape via are both declared and checked against the process (#136, #138). Fanout *strategy* is not: KiCadRoutingTools ships a BGA fanout engine with an under-pad method for arrays like this one, and Synth does not invoke it (#137). |
| DDR or equivalent length-sensitive interface | **No.** No DRAM part exists, and `match_length` is one tolerance per class, so per-byte-group budgets are not expressible. |
| PCIe / USB 3.x / HDMI-class differential | **Declared and checked, not met.** Four TMDS pairs at 100 Ω with a skew budget, and a length-matched class the routed copper fails — see *Limitations*. |
| Multi-rail compute, thermal, keepout, DFM | **Partly.** Both rails route as inner-layer planes; thermal and sequencing are not modelled. |

## Reproducibility

The sidecar pins all 49 components, and three consecutive exports produce a
byte-identical board.

Two of those coordinates describe the request rather than the result. Edge
legalisation runs after a `hard` override and moves J1 by −0.56 mm in y and J2
by +0.725 mm in x — the same step every run, whatever they are pinned to
(#133). Because the offset is constant the board stays deterministic, which is
what rerun stability needs; what it costs is that the sidecar is not a literal
description of where those two connectors end up. A true fixed point, where the
pinned value is reproduced verbatim, is not reachable for them while the offset
is non-zero.

Routing is deterministic for a given board name: the name seeds the entity
uuids, so renaming changes the order the router works in. Compare connectivity,
not via counts.
