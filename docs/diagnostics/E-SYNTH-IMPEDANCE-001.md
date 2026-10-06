# E-SYNTH-IMPEDANCE-001 — declared trace width is off the impedance target

**Severity:** error (more than 20 percent off), warning (10 to 20 percent off)
**Stage:** erc — rf

## What this means

A single-ended controlled-impedance net (a `diff_pair` whose two legs are the same net, with an `impedance`) is joined to a `netclass` with a `trace_width`, and the board declares a `stackup`. Synth computes the impedance that width gives as an outer-layer microstrip over the stackup's first insulator, and compares it with the target. More than 20 percent off is an error; 10 to 20 percent off is a warning.

The result is an estimate (about 10 percent) from a closed-form formula. It does not replace expert analysis. A board without a stackup is not checked.

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

Use the width that gives the target, or drop the `class` join (or the class's `trace_width`) and let the exporter derive the width from the stackup and the target.
