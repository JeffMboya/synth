# E-SYNTH-IMPEDANCE-001 — declared trace geometry is off the impedance target

**Severity:** error (more than 20 percent off), warning (10 to 20 percent off)
**Stage:** erc — rf

## What this means

A single-ended controlled-impedance net (a `diff_pair` whose two legs are the same net, with an `impedance`) is joined to a `netclass` with a `trace_width`, and the board declares a `stackup`. Synth computes the impedance that width gives as an outer-layer microstrip over the stackup's first insulator, and compares it with the target. More than 20 percent off is an error; 10 to 20 percent off is a warning.

The result is an estimate (about 10 percent) from a closed-form formula. It does not replace expert analysis. A board without a stackup is not checked.

A differential pair (a `diff_pair` whose two legs are different nets, with an `impedance`) is checked the same way on its declared `trace_width` and `clearance`, the clearance being the gap between the two traces. Its differential impedance is estimated from the edge-coupled outer-layer microstrip approximation Zdiff = 2 * Z0 * (1 - 0.48 * exp(-0.96 * gap / height)); its accuracy is not characterised here, so the result is an estimate that does not replace expert analysis. A class with a `trace_width` but no `clearance` is evaluated at the manufacturer minimum clearance, the tightest gap the exporter derives from. Both legs are checked: legs that resolve to different declared geometry, or only one leg that declares a width, are an error, with or without a `stackup`; the exporter withholds the pair settings for such a pair either way.

## Minimal reproduction

```synth
board "x" {
  layers 2
  netclass "RF" { trace_width 0.12mm }
  component U1: antenna "ant_chip_2g4"
  component R1: resistor "r_generic_0603" value "10k"
  connect U1.feed -> R1.p1 as "RF_IN" class "RF"
  diff_pair u1_feed r1_p1 { impedance 50ohm }
  stackup {
    copper 0.035mm
    insulator 0.2104mm er 4.4
    copper 0.035mm
  }
}
```

## Suggested fix

Use the width that gives the target, or drop the `class` join (or the class's `trace_width`) and let the exporter derive the width, and for a differential pair the gap, from the stackup and the target. The derived gap is written to the exported net class as `diff_pair_gap`.
