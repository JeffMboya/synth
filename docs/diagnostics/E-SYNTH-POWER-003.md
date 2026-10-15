# E-SYNTH-POWER-003 — power input without a source

**Severity:** error
**Stage:** erc — power

## What this means

A `power_input` pin sits on a net with nothing on it that could be supplying
power.

Three things count as a supply:

- a `power_output` pin — a regulator's output, a charger's;
- a `connector`, `jack` or `receptacle`, because the net leaves the board and
  what drives it there is not in the source;
- a passive whose **other** terminal is on a net that is neither this one nor a
  ground — a ferrite bead, a fuse or a series resistor feeding a rail from
  another rail.

A decoupling capacitor is not a supply, however many of them a net carries:
its far plate is on ground, so it brings no power in from anywhere. Treating
every passive as a supply made this rule unable to fire on any board that
satisfies `E-SYNTH-POWER-001`, which is the rule that requires the decoupling.

## Minimal reproduction

```synth
board "x" {
  component U1: mcu "stm32g030"
  component U2: mcu "stm32g030"
  connect U1.vdd -> U2.vdd  // no supply anywhere
}
```

The same board with a 100 nF from `vdd` to `vss` still reports it.

## Suggested fix

Wire the rail to a regulator output, a battery, or an external power
connector. A capacitor on the rail is decoupling, not a feed.
