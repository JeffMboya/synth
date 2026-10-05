# E-SYNTH-STACKUP-001 — stackup does not match `layers`

**Severity:** error
**Stage:** erc — board

## What this means

Two rules share this code:

- The number of `copper` entries in a `stackup` block must equal the board's `layers`. The two describe the same board, so one of them is wrong; Synth does not guess which, and does not export a stackup that contradicts the layer count.
- A board with a `stackup` must have 2, 4 or 6 layers. Those are the layer counts the exporter writes copper layer names for, so a stackup on any other count would name layers the board file does not have.

## Minimal reproduction

```synth
board "x" {
  layers 4
  stackup {
    copper 0.035mm
    insulator 1.5mm er 4.4
    copper 0.035mm
  }
}
```

## Suggested fix

Either change `layers` to the number of `copper` entries, or list every copper layer of the intended board:

```synth
layers 2
```

A four-layer stackup has four `copper` entries separated by three `insulator` entries. For a board with a different layer count, drop the `stackup` block.
