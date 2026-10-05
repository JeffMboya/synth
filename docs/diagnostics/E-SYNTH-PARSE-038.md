# E-SYNTH-PARSE-038 — insulator without a dielectric constant

**Severity:** error
**Stage:** parse

## What this means

An `insulator` entry in a `stackup` block was not followed by `er <number>`, or the value after `er` was not a plain number. The dielectric constant (relative permittivity) is a bare number such as `4.2`; it takes no unit. Synth has no default material, so an insulator without one is incomplete rather than guessed.

## Minimal reproduction

```synth
board "x" {
  layers 2
  stackup {
    copper 0.035mm
    insulator 1.5mm
    copper 0.035mm
  }
}
```

## Suggested fix

```synth
insulator 1.5mm er 4.2
```

Take the value from the laminate datasheet or the fabricator's stackup sheet for the frequency you care about.
