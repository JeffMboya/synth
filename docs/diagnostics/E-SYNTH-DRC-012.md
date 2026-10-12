# E-SYNTH-DRC-012 — routed geometry is off the impedance target

**Severity:** error
**Stage:** drc

## What this means

A net or differential pair declares an `impedance` target — on a `diff_pair`
or on a `netclass` — and a `stackup` is present, but the copper actually
routed for it is the wrong width for that impedance on the layer it ran on.

For a differential pair the estimate uses the edge-coupled microstrip model
and the declared (or manufacturer-minimum) gap. A single-ended target — a
`diff_pair` whose two halves are the same net, or a `netclass` `impedance` —
uses the microstrip or stripline model of the layer the copper is on. The
routed width is taken as the one carrying the most length. More than 20
percent off the target is an error.

This is the post-route counterpart of the compile-time `E-SYNTH-IMPEDANCE-001`:
here the copper the router emitted is measured, not what the design asked for.
A board without a stackup is not checked (see `E-SYNTH-STACKUP-005`).

## Minimal reproduction

```synth
board "b" {
  layers 4
  stackup {
    copper 0.035mm
    insulator 0.2104mm er 4.4
    copper 0.0152mm
    insulator 1.065mm er 4.6
    copper 0.0152mm
    insulator 0.2104mm er 4.4
    copper 0.035mm
  }
  netclass "RF" {
    impedance 50ohm
  }
  # … a member net routed far narrower than 50 ohm needs …
}
```

## Suggested fix

Route the net at the width the stackup demands — the violation names it — or
change the target. The width is a function of the dielectric height and
permittivity of the layer, so a different stackup reaches the same target with
a different width.
