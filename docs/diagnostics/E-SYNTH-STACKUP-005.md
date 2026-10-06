# E-SYNTH-STACKUP-005 — controlled impedance not verified, no stackup

**Severity:** warning
**Stage:** erc — board

## What this means

A `diff_pair` with an `impedance` target is a controlled-impedance net, and the design has no `stackup` block. Without the stackup Synth cannot say whether the target is reachable, so it reports the impedance as **not verified** rather than passing it or guessing a result. The warning does not block the build.

One warning is emitted per controlled-impedance pair, including a pair whose two legs name the same net (an impedance target on a single-ended RF net). A one-layer board gets E-SYNTH-STACKUP-004 instead.

## Minimal reproduction

```synth
board "x" {
  layers 4
  component R1: resistor "r_generic_0603" value "10k"
  component R2: resistor "r_generic_0603" value "10k"
  connect R1.p1 -> R2.p1
  connect R1.p2 -> R2.p2
  diff_pair r1_p1 r1_p2 { impedance 90ohm }
}
```

## Suggested fix

Declare the board's stackup, taken from the fabricator's stackup sheet, with one `copper` entry per layer, for example for `layers 2`:

```synth
stackup {
  copper 0.035mm
  insulator 1.5mm er 4.4
  copper 0.035mm
}
```
