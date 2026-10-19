# E-SYNTH-CLOCK-002 — clock input without source

**Severity:** error
**Stage:** erc — clock

## What this means

A `clock_input` capability is declared on a net that has no `clock_output` pin nor any passive component that could supply a crystal-derived clock.

A net that reaches a connector, jack or receptacle is not reported: it leaves
the board, and what drives it on the other side is not in the source. That is
the normal shape of a mezzanine, a memory card or any daughterboard clocked by
its carrier.

## Minimal reproduction

```synth
board "x" {
  component U1: ic "sn74hc595_shift"
  component U2: ic "sn74hc165_pload"
  connect U1.srclk -> U2.clk  // both clock inputs, nothing drives
}
```

## Suggested fix

Wire the net to an oscillator, crystal, or MCU clock-output pin — or to the
connector the clock actually arrives on.
