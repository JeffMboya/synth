# E-SYNTH-POWER-011 — regulator feedback is taken from the switching node

**Severity:** error
**Stage:** erc — power

## What this means

A pin tagged `feedback` (the output-sense input of a switching regulator) is
connected to a net that carries a pin tagged `switch_node`, either directly or
through exactly one two-terminal resistor. The switch node swings between the
input rail and ground at the switching frequency, so a divider tapped from it
feeds the regulator a chopped waveform instead of the regulated output. The
loop regulates the wrong quantity and the output is unstable or wrong.

Both roles come from the part's registry `capabilities` (`switch_node` and
`feedback`), never from pin names. A regulator part that declares neither tag
is not checked.

The rule reports one diagnostic per feedback pin and stops at the first match.
It follows one resistor only; longer paths are not traced.

## Minimal reproduction

```synth
board "x" {
  layers 4
  component U1: regulator "mp2307_buck"
  component L1: inductor "l_generic_1210" value "10uH"
  component R1: resistor "r_generic_0402" value "31.6k"
  component R2: resistor "r_generic_0402" value "10k"
  connect U1.sw -> L1.p1
  connect U1.sw -> R1.p1
  connect R1.p2 -> U1.fb
  connect U1.fb -> R2.p1
  connect R2.p2 -> U1.gnd
}
```

## Suggested fix

Tap the divider from the regulated output instead, on the far side of the
inductor:

```synth
connect L1.p2 -> R1.p1
```
