# W-SYNTH-IMPEDANCE-001 — impedance not verified on inner layers

**Severity:** warning
**Stage:** erc — rf

## What this means

A single-ended controlled-impedance net is on a board whose `stackup` has inner copper layers. The router may place the net on an inner layer, and Synth derives and checks only outer-layer microstrip width, so the impedance there is not verified.

The estimate Synth does make (about 10 percent) does not replace expert analysis.

## Minimal reproduction

```synth
board "x" {
  layers 4
  component U1: antenna "ant_chip_2g4"
  component R1: resistor "r_generic_0603" value "10k"
  connect U1.feed -> R1.p1
  diff_pair u1_feed r1_p1 { impedance 50ohm }
  stackup {
    copper 0.035mm
    insulator 0.2104mm er 4.4
    copper 0.0152mm
    insulator 1.065mm er 4.6
    copper 0.0152mm
    insulator 0.2104mm er 4.4
    copper 0.035mm
  }
}
```

## Suggested fix

Have the impedance of the routed net reviewed on the layer it uses, or use a two-layer board where every trace is outer-layer microstrip.
