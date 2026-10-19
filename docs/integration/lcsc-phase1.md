# LCSC/JLCPCB integration — Phase 1 implementation

Phase 1 turns the Phase 0 contract into a reliable supply boundary in `synth-supply`.

## Implemented contract

- LCSC results are accepted only when the requested normalized C-number exactly matches a returned product code. A search with multiple nonmatching candidates returns `AmbiguousMatch`; a single nonmatching/empty result returns `NoExactMatch`. The previous first-result fallback is gone.
- HTTP failures and non-success provider responses return `ProviderUnavailable`. They are no longer represented as an absent part or zero stock.
- Missing inventory is represented by `StockStatus::Unknown`; an explicit zero is `OutOfStock`. Missing lifecycle is `LifecycleStatus::Unknown`; missing MOQ remains unknown through `PurchaseConstraints::moq = None`.
- `AssemblyEligibility` is independent from inventory and defaults to `Unknown`; a stocked part is never implicitly treated as JLCPCB-assembly eligible.
- `PurchaseConstraints` carries MOQ, order multiple, and all known quantity price tiers. Legacy fields remain in `SupplyStatus` for compatibility and are populated only where the provider supplied a value.
- Part numbers are normalized for cache keys and BOM requests. `query_bom` de-duplicates repeated normalized provider requests while returning the result for every reference.
- `SupplyCache::put_snapshot` persists a complete status set in one SQLite transaction and stamps every row with a shared snapshot ID. Existing caches are upgraded with checked additive migrations; migration errors are returned before writes.

## Evidence

`crates/synth-supply/tests/supply_tests.rs` covers exact identity, ambiguity rejection, unknown inventory/lifecycle/MOQ, provider request de-duplication, and atomic snapshot metadata. The Phase 0 JSON fixtures remain the deterministic response corpus.

Verification performed for this change:

- `cargo test -p synth-supply --locked`: 10 passed
- `cargo test -p synth-validate --test supply_chain_tests --locked`: 1 passed (with the cache migration permitted to write the existing local cache)
- `cargo clippy -p synth-supply --all-targets --locked -- -D warnings`: passed

The remaining catalog ingestion, procurement lock, and supplier eligibility model are intentionally Phase 2/3 work. This phase provides the reliable identity/error/cache substrate they consume.
