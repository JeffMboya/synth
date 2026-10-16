# LCSC/JLCPCB integration — Phase 0 audit

Status: complete for the pinned audit scope.

This document is the Phase 0 compatibility record for Synth core. It is intentionally a contract and evidence record; it does not claim that live supplier lookup or ordering is supported yet.

## Pinned upstream reference

| Field | Value |
| --- | --- |
| Repository | `https://github.com/Bouni/kicad-jlcpcb-tools` |
| Commit | `14bf352cc9b1b953aa781c49d9a32805aeb72bd0` |
| Project version at audit | `2025.09.02` (`pyproject.toml`) |
| Code license | MIT; retain the copyright and permission notice for copied substantial portions |
| Runtime | Python >= 3.10; plugin dependencies include requests, cachetools, ratelimit, retry, and split_file_reader |
| Audit date | 2026-10-16 |

The commit is recorded instead of tracking `main`, because catalog schema, pricing behavior, and export details can change independently.

## Scope inspected

The audit inspected these upstream areas and their associated tests:

- `bom_estimation/pricing.py` and `tests/test_bom_estimator_controller.py`: quantity-tier parsing, component aggregation, Economic/Standard assembly fees, extended-part surcharges, missing-price diagnostics, and per-board totals.
- `db_build/`, `dblib/`, `library.py`, and `common/translate.py`: catalog database generation, local database state, price-band representation, and stock/catalog update flow.
- `bom_csv.py`, `tests/test_bom_csv.py`, `tests/test_bom_manufacturer_columns.py`, and `tests/test_bom_designator_split.py`: JLC BOM columns, manufacturer fields, CSV quoting, and the 2048-byte line limit.
- `fabrication.py`, `tests/test_fabrication_cpl_format.py`, `tests/test_fabrication_position.py`, and `tests/test_fabrication_corrections.py`: origin/coordinate transforms, bottom-side rotation, correction application, and CPL output.
- `CORRECTIONS.md`, `tests/test_correction_data.py`, `tests/test_corrections_import_export.py`, and `tests/test_library_lcsc_corrections.py`: exact-LCSC correction precedence, validation, persistence, and the fact that pattern CSV does not carry exact-part rules.
- `LICENSE`, `pyproject.toml`, and `README.md`: reuse terms, supported runtime, installation, and plugin boundaries.

## Reuse decision

Synth will not vendor the plugin or make its GUI/runtime a SaaS dependency. The following are interoperability references only:

| Upstream behavior | Synth Phase 0 decision |
| --- | --- |
| BOM grouping and 2048-byte designator splitting | Reimplement as a Rust export profile, with contract fixtures and explicit JLC column names. |
| CPL coordinate and correction rules | Reimplement only after defining Synth's origin, axis, side, and rotation contract. Keep exact-LCSC correction keys distinct from pattern keys. |
| Catalog database/build pipeline | Do not copy catalog data yet. Confirm source terms and redistribution rights before any snapshot is shipped. |
| `bom_estimation` pricing | Use as a behavioral reference and comparison fixture. Do not copy fee defaults or floating-point arithmetic; Synth will use versioned decimal/integer-scaled money and current provider rules. |
| Plugin GUI, SQLite project state, and automatic schematic writes | Not a runtime dependency. Desktop round-trip support will be explicit and diffable. |

The upstream estimator's dated fee defaults are useful for identifying fee categories, but they are not a current quote. JLCPCB's current fee schedule must be represented by a dated Synth policy and compared with a supplier quote before release.

## Existing Synth audit

- `synth-registry` already carries MPN, LCSC number aliases, KiCad symbol/footprint references, dimensions, operating conditions, and provenance/review metadata.
- `synth-supply` already has LCSC/Nexar adapters and a SQLite cache. The current LCSC adapter's first-result fallback and defaulting missing stock/lifecycle fields require Phase 1 changes; this audit deliberately does not change those semantics.
- `synth-kicad` already exports a general BOM and PnP CSV and omits DNP parts. The PnP exporter currently skips missing placements and maps inner layers to Bottom; strict assembly validation is a later phase.
- `synth-kicad` schematic export emits `MPN` and `LCSC` properties. PCB field round-trip and alias conflict handling remain Phase 2 work.
- The CLI already has supply-chain querying and `kicad-cli` fabrication gates. This plan extends those contracts rather than creating a parallel supply implementation.

## Fixture contract

The sanitized, deterministic supplier and manufacturing fixtures are under:

```text
crates/synth-supply/tests/fixtures/lcsc-phase0/
fixtures/lcsc-phase0/manufacturing/
```

They cover exact identity, ambiguous search, unavailable provider, DNP filtering, grouped references, and the expected BOM/CPL reference set. They contain no credentials and are not assertions about current stock or price.

## Phase 0 exit criteria

- [x] Upstream revision is pinned and its license recorded.
- [x] Catalog, estimator, correction, BOM, CPL, and relevant test areas are listed.
- [x] Reuse versus reimplementation decisions are documented.
- [x] Existing registry, supply, BOM, PnP, and fabrication gaps are recorded.
- [x] Deterministic supplier responses and expected manufacturing files are checked in.
- [x] Live provider access is not required for CI; live smoke tests remain opt-in.

