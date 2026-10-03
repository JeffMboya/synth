# E-SYNTH-DIFF-004 — net is a leg of more than one differential pair

**Severity:** error
**Stage:** erc — connectivity

## What this means

A net is a leg of two different `diff_pair` declarations. This usually means a pair was declared twice, or two pairs were pointed at the same copper by different names (for example `U1_usb_dp` and `J1_dp`). A net cannot be one half of two pairs, so the pair constraints (impedance, length matching) would conflict.

A single `diff_pair` that names one net twice is not reported. That form is how a single-ended RF net gets an impedance target (see `E-SYNTH-RF-003`).

## Minimal reproduction

```synth
board "x" {
  component J1: connector "usb_c_receptacle"
  component U1: mcu "rp2350"
  connect J1.dp -> U1.usb_dp
  connect J1.dn -> U1.usb_dn
  diff_pair j1_dp j1_dn { impedance 90ohm }
  diff_pair j1_dp u1_usb_dn { impedance 90ohm }
}
```

## Suggested fix

Remove the duplicate `diff_pair`, or point each pair at its own nets.
