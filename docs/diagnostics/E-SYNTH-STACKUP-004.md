# E-SYNTH-STACKUP-004 — controlled impedance has no reference plane

**Severity:** error
**Stage:** erc — board

## What this means

A `diff_pair` with an `impedance` target is a controlled-impedance net: its trace width is only meaningful over a reference plane. A one-layer board has only the signal layer, so there is no plane for the target to refer to. A pair whose two legs name the same net (an impedance target on a single-ended RF net) is checked the same way.

The rule judges the declared intent: it flags `layers 1`, a board with no second copper layer at all. It does not check that the adjacent layer of the exported board is a ground plane. The exporter only pours a ground zone for a ground net with at least three endpoints, so a design without one can still export with no reference plane. Inner-layer traces are not modelled.

## Minimal reproduction

```synth
board "x" {
  layers 1
  component R1: resistor "r_generic_0603" value "10k"
  component R2: resistor "r_generic_0603" value "10k"
  connect R1.p1 -> R2.p1
  connect R1.p2 -> R2.p2
  diff_pair r1_p1 r1_p2 { impedance 90ohm }
}
```

## Suggested fix

Use a board with a second copper layer for the plane, for example `layers 2` or `layers 4`, or remove the `impedance` target if the pair does not need controlled impedance:

```synth
layers 4
```
