# E-SYNTH-DRC-014 — differential pair exceeds its skew budget

**Severity:** error
**Stage:** drc

## What this means

A `diff_pair` declares a `max_skew`, and the routed copper on its two halves
differs in length by more than that. Skew is measured from the emitted
segments, so it holds regardless of what the router reported about its own
length matching: the two halves must reach their destination within the budget
the design set.

The length of each half is the total of its routed segments. A half with no
copper is not counted — an unrouted net is `E-SYNTH-ROUTE-001`'s concern — so
this rule fires only once both halves carry copper.

## Minimal reproduction

```synth
board "b" {
  layers 4
  diff_pair U1_usb_dp U1_usb_dn {
    impedance 90ohm
    max_skew 0.15mm
  }
  # … routes where the positive half is 0.4 mm longer than the negative …
}
```

## Suggested fix

Lengthen the shorter half with a serpentine or detour until the two match
within the budget, or relax `max_skew` if the interface tolerates more.
