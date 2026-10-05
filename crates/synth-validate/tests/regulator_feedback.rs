// SPDX-License-Identifier: Apache-2.0

//! Message text and boundaries of `E-SYNTH-POWER-011`. The pass/fail fixture pairs
//! live in `fixtures/erc/`.

use std::path::{Path, PathBuf};

use synth_diagnostics::Diagnostic;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn fixture(name: &str) -> String {
    let path = workspace_root()
        .join("fixtures")
        .join("erc")
        .join(format!("{name}.synth"));
    std::fs::read_to_string(path).expect("fixture must exist")
}

fn diagnostics(src: &str) -> Vec<Diagnostic> {
    let filename = "regulator_feedback.synth".to_string();
    let parse = synth_parser::parse(src, filename.clone());
    let ast = parse.ast.as_ref().expect("source must parse");
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("registry must load");
    let lowered = synth_ir::lower(ast, &registry, &filename);
    let board = lowered.board.as_ref().expect("board must lower");
    synth_validate::run_erc(board, &filename)
}

fn found(src: &str) -> Option<String> {
    diagnostics(src)
        .into_iter()
        .find(|d| d.code == "E-SYNTH-POWER-011")
        .and_then(|d| d.found)
}

#[test]
fn a_feedback_pin_on_the_switch_net_says_it_shares_the_net() {
    let found = found(&fixture("E-SYNTH-POWER-011__feedback_on_switch_net")).expect("flagged");
    assert!(
        found.contains("shares net") && found.contains("switch-node pin"),
        "{found}"
    );
}

#[test]
fn a_divider_tapped_from_the_switch_node_names_the_resistor() {
    let found = found(&fixture("E-SYNTH-POWER-011__divider_on_switch_node")).expect("flagged");
    assert!(found.contains("joined through resistor `R1`"), "{found}");
}
