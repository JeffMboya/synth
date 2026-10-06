// SPDX-License-Identifier: Apache-2.0

//! End-to-end tests for `E-SYNTH-STACKUP-004` (no reference plane) and
//! `E-SYNTH-STACKUP-005` (impedance not verified, no stackup). The plain
//! pass/fail pairs also live in `fixtures/erc/`; the tests here pin the message
//! text and the edge cases fixtures cannot express.

use std::path::{Path, PathBuf};

use synth_diagnostics::{Diagnostic, Severity};

const NO_PLANE: &str = "E-SYNTH-STACKUP-004";
const NOT_VERIFIED: &str = "E-SYNTH-STACKUP-005";

fn registry_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("registry")
        .join("parts")
}

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join("erc")
        .join(format!("{name}.synth"));
    std::fs::read_to_string(path).expect("fixture must exist")
}

fn diags(src: &str) -> Vec<Diagnostic> {
    let filename = "impedance_reference.synth".to_string();
    let parse = synth_parser::parse(src, filename.clone());
    let ast = parse.ast.as_ref().expect("source must parse");
    let registry = synth_registry::load_dir(&registry_dir()).expect("registry must load");
    let lowered = synth_ir::lower(ast, &registry, &filename);
    let board = lowered.board.as_ref().expect("board must lower");
    synth_validate::run_erc(board, &filename)
}

fn with_code(src: &str, code: &str) -> Vec<Diagnostic> {
    diags(src).into_iter().filter(|d| d.code == code).collect()
}

const STACKUP_2: &str = "stackup {
    copper 0.035mm
    insulator 1.5mm er 4.4
    copper 0.035mm
  }";

const RESISTOR_NETS: &str = "component R1: resistor \"r_generic_0603\" value \"10k\"
  component R2: resistor \"r_generic_0603\" value \"10k\"
  connect R1.p1 -> R2.p1
  connect R1.p2 -> R2.p2";

fn board(layers: u32, stackup: &str, pairs: &str) -> String {
    format!("board \"t\" {{\n  layers {layers}\n  {RESISTOR_NETS}\n  {pairs}\n  {stackup}\n}}\n")
}

#[test]
fn one_layer_pair_names_the_missing_reference_plane() {
    let found = with_code(&fixture("E-SYNTH-STACKUP-004__one_layer"), NO_PLANE);
    assert_eq!(found.len(), 1);
    let d = &found[0];
    assert_eq!(d.severity, Severity::Error);
    assert!(d.title.contains("reference plane"), "{}", d.title);
    let text = d.found.as_deref().unwrap();
    assert!(
        text.contains("one-layer board has no reference plane"),
        "{text}"
    );
    assert!(text.contains("90"), "{text}");
}

#[test]
fn one_layer_pair_is_not_also_reported_as_not_verified() {
    assert!(with_code(&fixture("E-SYNTH-STACKUP-004__one_layer"), NOT_VERIFIED).is_empty());
}

#[test]
fn pair_without_stackup_is_warned_not_verified() {
    let src = fixture("E-SYNTH-STACKUP-005__no_stackup");
    let found = with_code(&src, NOT_VERIFIED);
    assert_eq!(found.len(), 1);
    let d = &found[0];
    assert_eq!(d.severity, Severity::Warning);
    assert!(d.title.contains("not verified"), "{}", d.title);
    assert!(d.found.as_deref().unwrap().contains("not verified"));
    assert!(with_code(&src, NO_PLANE).is_empty());
}

#[test]
fn pair_with_stackup_is_not_reported_as_not_verified() {
    let src = board(2, STACKUP_2, "diff_pair r1_p1 r1_p2 { impedance 90ohm }");
    assert!(with_code(&src, NOT_VERIFIED).is_empty());
    assert!(with_code(&src, NO_PLANE).is_empty());
}

#[test]
fn single_ended_rf_idiom_pair_is_a_controlled_impedance_net() {
    let src = "board \"t\" {
  layers 4
  component U1: antenna \"ant_chip_2g4\"
  component R1: resistor \"r_generic_0603\" value \"10k\"
  connect U1.feed -> R1.p1
  diff_pair u1_feed u1_feed { impedance 50ohm }
}
";
    assert_eq!(with_code(src, NOT_VERIFIED).len(), 1);
    let src = src.replace("layers 4", "layers 1");
    assert_eq!(with_code(&src, NO_PLANE).len(), 1);
}

#[test]
fn each_controlled_impedance_pair_gets_its_own_diagnostic() {
    let src = "board \"t\" {
  layers 1
  component R1: resistor \"r_generic_0603\" value \"10k\"
  component R2: resistor \"r_generic_0603\" value \"10k\"
  component R3: resistor \"r_generic_0603\" value \"10k\"
  component R4: resistor \"r_generic_0603\" value \"10k\"
  connect R1.p1 -> R2.p1
  connect R1.p2 -> R2.p2
  connect R3.p1 -> R4.p1
  connect R3.p2 -> R4.p2
  diff_pair r1_p1 r1_p2 { impedance 90ohm }
  diff_pair r3_p1 r3_p2 { impedance 100ohm }
}
";
    let found = with_code(src, NO_PLANE);
    assert_eq!(found.len(), 2);
    assert!(found[0].found.as_deref().unwrap().contains("90"));
    assert!(found[1].found.as_deref().unwrap().contains("100"));
}

#[test]
fn pair_without_an_impedance_target_is_untouched() {
    for layers in [1, 2] {
        let src = board(layers, "", "diff_pair r1_p1 r1_p2");
        assert!(with_code(&src, NO_PLANE).is_empty(), "layers {layers}");
        assert!(with_code(&src, NOT_VERIFIED).is_empty(), "layers {layers}");
    }
}
