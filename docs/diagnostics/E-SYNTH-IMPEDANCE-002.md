# E-SYNTH-IMPEDANCE-002 — impedance target is outside the range this estimate covers

**Severity:** error
**Stage:** erc — rf

## What this means

A single-ended controlled-impedance net on a board with a `stackup` declares an `impedance` outside the range this estimate covers: no outer-layer microstrip width in that range gives it. The widths considered run from the manufacturer minimum trace width (or 0.1 times the dielectric height, if larger) up to 2 times the dielectric height, the range the closed-form formula is stated for (IPC-2141). The message gives the impedance range that results (lowest to highest, on this stackup).

The result is an estimate (about 10 percent) and does not replace expert analysis. No controlled-impedance class is derived: the net keeps its declared class, or falls back to the default buckets.

## Minimal reproduction

```synth
board "x" {
  layers 2
  component U1: antenna "ant_chip_2g4"
  component R1: resistor "r_generic_0603" value "10k"
  connect U1.feed -> R1.p1
  diff_pair u1_feed r1_p1 { impedance 5ohm }
  stackup {
    copper 0.035mm
    insulator 0.2104mm er 4.4
    copper 0.035mm
  }
}
```

## Suggested fix

Choose a target inside the range, or change the stackup: a thicker dielectric under the outer layer reaches lower impedances with wider traces and a thinner one reaches them with narrower traces.
