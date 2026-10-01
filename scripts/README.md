# Synth Scripts Directory

This directory contains developer tooling for fixtures, registry maintenance, and reference exports.

---

## Anomaly Detector Training (`train_anomaly_model.py`)

Trains the **One-Class SVM Graph Anomaly Detector** (`W-SYNTH-ANOMALY-001`) used by `synth-validate`.

### Overview

- **Input:** Clean SynthSpec design fixtures (`pass__*.synth` under `fixtures/erc/` and passing designs under `fixtures/designs/`).
- **Feature Extraction:** 15 graph-level structural metrics (component count, net degree, passive ratio, power pin ratio, decoupling density, protocol pin counts, etc.) extracted via `dump_features.rs`.
- **Model:** `StandardScaler` + `OneClassSVM(kernel='rbf', nu=0.05, gamma=0.001)` from `scikit-learn`.
- **Output:** `crates/synth-validate/src/anomaly_model.json` (embedded at compile-time in Rust via `include_str!`).

### Running the Trainer

```bash
python3 scripts/train_anomaly_model.py
```

### Re-Training Cadence & Invariants

> [!IMPORTANT]
> **When to Re-Train:**
>
> 1. **New Fixtures Added:** Whenever new clean reference designs are added to `fixtures/designs/` or `fixtures/erc/pass__*.synth`.
> 2. **Registry Expansion:** When major component categories or capabilities are added to `registry/parts/`.
> 3. **Larger design coverage:** When production-scale designs are added to the codebase.

> [!NOTE]
> **Verification Gate:** The script enforces that the held-out false-positive rate (FPR) on 10 held-out clean fixtures is **< 5.0%**. If a re-training attempt fails this gate, the script aborts without modifying `anomaly_model.json`.

---

## Schematic Quality Measurement (`measure_schematic_quality.sh`)

Scores the auto-layout of every design that lowers, using
`synth layout <file> --score`, and prints one TSV row per design:

```text
design        crossings  wire_len_mm  label_stubs
```

It exists because `fixtures/layout/score_baselines.json` covers only
**10 designs, all of which score zero crossings** — so the
`scorer_gate` test asserts `< 5` against a baseline of `0` and
therefore cannot fail. This script widens the corpus to ~52 designs
across `fixtures/designs/`, `examples/` and `fixtures/kicad-reference/`,
which is where a layout change actually shows up.

### Usage

Measure the current tree, then compare against a baseline:

```bash
cargo build --release -p synth-cli
./scripts/measure_schematic_quality.sh > after.tsv

git stash                 # apply the change under test to HEAD
cargo build --release -p synth-cli
./scripts/measure_schematic_quality.sh > before.tsv

diff -u before.tsv after.tsv
```

### Reading the result

- **`crossings` is 0 nearly everywhere.** The scorer's crossing count
  and the `E-SYNTH-SCHEM-002` rule disagree, and neither is currently
  sensitive enough to be a useful signal. Treat changes in
  `wire_len_mm` and `label_stubs` as the meaningful ones.
- **`label_stubs` rising while `wire_len_mm` falls** is the expected
  trade: a long wire became a pair of labels. Whether that is an
  improvement depends on the design, so read it with the render.
- A design that vanishes from the output stopped lowering — check
  stderr rather than assuming the layout changed.
