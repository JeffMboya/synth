# E-SYNTH-DRC-015 — trace too narrow for its declared current

**Severity:** error
**Stage:** drc

## What this means

A `netclass` declares a `current`, and a member net's routed width is below
the IPC-2221 width for that current over the layer's copper thickness. The
thickness comes from the board's `stackup` when one is present, and is assumed
to be 1 oz (0.035 mm) otherwise. A 10 degrees Celsius temperature rise is
assumed.

Only classes that declare a `current` are checked, so this never fires on a
design that has not asked for it. The estimate is a conservative sizing aid,
not a thermal simulation — it ignores copper planes, thermal vias and
neighbouring heat sources.

## Minimal reproduction

```synth
board "b" {
  layers 2
  netclass "PWR" {
    trace_width 0.25mm
    current 1.5A
  }
  # … a member net routed at 0.25 mm on 1 oz copper …
}
```

## Suggested fix

Widen the net — the violation names the width the current needs — add copper on
another layer, split the current across parallel vias and traces, or move the
net to a thicker copper layer.
