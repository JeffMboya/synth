# E-SYNTH-STACKUP-003 — stackup thickness or dielectric constant out of range

**Severity:** error
**Stage:** erc — board

## What this means

A layer in a `stackup` block has a thickness of zero or less, or an insulator has a dielectric constant (`er`) below 1. Neither is physical: a layer has positive thickness, and no material has a relative permittivity below that of vacuum.

## Minimal reproduction

```synth
board "x" {
  layers 2
  stackup {
    copper 0.035mm
    insulator 0mm er 4.4
    copper 0.035mm
  }
}
```

## Suggested fix

Give the layer its real thickness, taken from the fabricator's stackup sheet:

```synth
insulator 1.5mm er 4.4
```
