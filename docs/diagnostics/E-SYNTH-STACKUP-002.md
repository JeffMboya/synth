# E-SYNTH-STACKUP-002 — stackup layers do not alternate

**Severity:** error
**Stage:** erc — board

## What this means

The layers of a `stackup` block are listed from the top of the board to the bottom. They must start with `copper`, alternate with `insulator`, and end with `copper`: two coppers in a row would short, two insulators in a row are one insulator, and a board cannot end on bare dielectric.

## Minimal reproduction

```synth
board "x" {
  layers 2
  stackup {
    copper 0.035mm
    insulator 0.7mm er 4.4
    insulator 0.8mm er 4.4
    copper 0.035mm
  }
}
```

## Suggested fix

Merge consecutive insulators into one entry, or put the missing copper layer between them:

```synth
stackup {
  copper 0.035mm
  insulator 1.5mm er 4.4
  copper 0.035mm
}
```
