# Phase 0 manufacturing fixture

This fixture defines the reference set shared by a grouped JLCPCB BOM and its CPL. It is intentionally small and deterministic; coordinates are in millimetres and use the Synth PnP contract currently emitted by `synth-kicad`.

- `bom-jlcpcb.csv` contains one grouped row per exact assembly identity.
- `cpl-jlcpcb.csv` contains one placement for every supplier-assembled reference.
- `expected.json` records set equality and DNP/manual-assembly exclusions that a strict exporter must validate.

This is an expected-file contract, not a supplier quote. It does not certify the footprint or current JLCPCB eligibility of the example parts.

