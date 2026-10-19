# Phase 0 supplier fixtures

These are sanitized, deterministic contract fixtures for the LCSC adapter audit. They are not current catalog snapshots and must not be used to infer live stock, lifecycle, or price.

| Fixture | Expected classification |
| --- | --- |
| `exact-match.json` | Exact requested C-number is present; select it. |
| `ambiguous-search.json` | Multiple candidates and no exact requested C-number; return unresolved/ambiguous. |
| `provider-unavailable.json` | Transport/provider failure; return unavailable, not zero stock. |

The fixtures intentionally include missing stock/lifecycle information in the unavailable case. Missing values are unknown, not `stock_qty = 0` or `lifecycle = active`.

