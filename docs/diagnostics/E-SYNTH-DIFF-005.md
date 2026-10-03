# E-SYNTH-DIFF-005 — differential-pair leg on a power or ground net

**Severity:** error
**Stage:** erc — protocol

## What this means

A `diff_pair` leg resolves to a net that carries a power input, power output or ground pin. A differential pair is two signal nets; declaring one on a supply or ground rail means the pair constraint (impedance, length matching) is attached to the wrong copper.

## Minimal reproduction

```synth
board "x" {
  component J1: connector "usb_c_receptacle"
  component U1: mcu "rp2350"
  connect J1.dp -> U1.usb_dp
  connect J1.gnd -> U1.gnd
  diff_pair j1_dp j1_gnd { impedance 90ohm }
}
```

## Suggested fix

Name the two signal nets that make up the pair, for example `j1_dp j1_dn`.
