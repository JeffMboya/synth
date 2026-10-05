# E-SYNTH-PARSE-002 — expected board name (quoted string)

**Severity:** error
**Stage:** parse

## What this means

The `board` keyword must be followed by a quoted string literal naming
the board. The parser found something else (or end-of-file) at the
position where the name was expected. The same code is reported wherever
a quoted name is expected, for example the `material` of a `stackup`
insulator (`material "FR4"`, not `material FR4`).

## Minimal reproduction

```synth
board {}
```

## Suggested fix

Insert a placeholder name:

```synth
board "unnamed" {}
```

Patch primitive: `insert_at` at the position immediately after `board`.
