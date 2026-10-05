// SPDX-License-Identifier: Apache-2.0

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

const SYNTH: &str = env!("CARGO_BIN_EXE_synth");

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

fn fixture_root() -> PathBuf {
    workspace_root().join("fixtures").join("registry-qualify")
}

struct Export {
    code: Option<i32>,
    stderr: String,
}

fn export_with(label: &str, part_dir: &Path, part_id: &str, extra: &[&str]) -> Export {
    let dir = std::env::temp_dir().join(format!("synth_qual_gate_{}_{label}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let design = dir.join("board.synth");
    std::fs::write(
        &design,
        format!("board \"gate\" {{\n  layers 2\n  component U1: mcu \"{part_id}\"\n}}\n"),
    )
    .expect("write design");

    let output = Command::new(SYNTH)
        .arg("export-kicad")
        .arg(&design)
        .arg("--registry")
        .arg(part_dir)
        .arg("--out")
        .arg(dir.join("out"))
        .arg("--gerbers")
        .args(extra)
        .env("KICAD_FOOTPRINT_DIR", fixture_root().join("footprints"))
        .output()
        .expect("run synth export-kicad");

    Export {
        code: output.status.code(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn mutation_dir(id: &str) -> PathBuf {
    fixture_root().join("mutations").join(id)
}

#[test]
fn force_does_not_wave_past_a_pin_map_mismatch() {
    let run = export_with(
        "forced_mismatch",
        &mutation_dir("mut_missing_pad"),
        "mut_missing_pad",
        &["--force"],
    );
    assert_eq!(run.code, Some(1), "stderr:\n{}", run.stderr);
    assert!(
        run.stderr.contains("E-SYNTH-QUAL-000"),
        "the structural gate must refuse:\n{}",
        run.stderr
    );
    assert!(
        run.stderr.contains("E-SYNTH-QUAL-001"),
        "the specific defect must be named:\n{}",
        run.stderr
    );
}

#[test]
fn an_undeclared_exposed_pad_blocks_fab_export() {
    let run = export_with(
        "orphan_pad",
        &mutation_dir("mut_undeclared_exposed_pad"),
        "mut_undeclared_exposed_pad",
        &["--force"],
    );
    assert_eq!(run.code, Some(1), "stderr:\n{}", run.stderr);
    assert!(
        run.stderr.contains("E-SYNTH-QUAL-002"),
        "an undeclared copper pad must block:\n{}",
        run.stderr
    );
    assert!(
        !run.stderr.contains("E-SYNTH-PIN-001"),
        "ERC does not catch this case, which is why the gate exists:\n{}",
        run.stderr
    );
}

#[test]
fn a_structurally_sound_part_passes_the_gate() {
    let run = export_with(
        "sound",
        &fixture_root().join("parts"),
        "qual_mcu_qfn",
        &["--force"],
    );
    assert!(
        !run.stderr.contains("E-SYNTH-QUAL-000"),
        "a sound part must not be refused:\n{}",
        run.stderr
    );
}
