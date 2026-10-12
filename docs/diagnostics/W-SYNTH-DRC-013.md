# W-SYNTH-DRC-013 — parallel traces closer than a dielectric height

**Severity:** warning
**Stage:** drc

## What this means

Two nets' traces run parallel on the same layer, closer to each other than one
dielectric height and for more than ten dielectric heights of run length. A
long, close parallel run couples more energy between the two nets than a short
one, which is the geometric signature of crosstalk.

The rule is advisory: how much coupling is acceptable depends on the
signalling scheme, edge rates and what the aggressor and victim nets carry, so
it cannot be decided from geometry alone. At most one warning is raised per net
pair, for the pair's worst run, so a dense board does not flood. As a `W-`
code it never blocks a release.

## Minimal reproduction

Two signals placed one clearance apart that share a long straight run on the
same layer of a board with a declared stackup.

## Suggested fix

Increase the spacing between the two nets — roughly three times the dielectric
height removes most edge coupling — shorten the parallel run, divert one net to
another layer, or place a ground trace or pour between them. Where coupling does
not matter (a slow GPIO beside another slow GPIO) the warning can be ignored.
