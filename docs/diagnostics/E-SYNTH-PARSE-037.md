# E-SYNTH-PARSE-037 — unknown stackup entry

**Severity:** error
**Stage:** parse

## What this means

A `stackup { … }` block contained something other than a `copper` or `insulator` entry. The block is an ordered list of physical layers, outermost copper first, and those are the only two kinds of layer it knows.

## Minimal reproduction

```synth
board "x" {
  layers 2
  stackup {
    copper 0.035mm
    prepreg 0.2mm er 4.2
    copper 0.035mm
  }
}
```

## Suggested fix

```synth
stackup {
  copper 0.035mm
  insulator 0.2mm er 4.2
  copper 0.035mm
}
```

Whether an insulator is a core or a prepreg is not part of the language; both are an `insulator` with a thickness and a dielectric constant.
