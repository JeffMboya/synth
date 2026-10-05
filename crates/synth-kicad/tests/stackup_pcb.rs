// SPDX-License-Identifier: Apache-2.0

//! A declared `stackup { … }` reaches the exported `.kicad_pcb` as a
//! `(stackup …)` section inside `(setup …)`; a design without one exports
//! no stackup and the fixed default board thickness.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn tempdir(label: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "synth-kicad-stackup-{label}-{}",
        std::process::id()
    ));
    if p.exists() {
        let _ = fs::remove_dir_all(&p);
    }
    p
}

const BODY: &str = r#"
  component R1: resistor "r_generic_0603" value "10k"
  component R2: resistor "r_generic_0603" value "10k"
  connect R1.p1 -> R2.p1
  connect R1.p2 -> R2.p2
"#;

const FOUR_LAYER_STACKUP: &str = r#"
  stackup {
    copper 0.035mm
    insulator 0.2104mm er 4.4 material "FR4"
    copper 1.4mil
    insulator 1.065mm er 4.6
    copper 0.0152mm
    insulator 0.2104mm er 4.4 material "FR4"
    copper 0.035mm
  }
"#;

fn export_pcb(label: &str, layers: u32, extra: &str) -> String {
    let source = format!("board \"{label}\" {{\n  layers {layers}\n{BODY}{extra}}}\n");
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("seed registry must load");
    let parsed = synth_parser::parse(&source, format!("{label}.synth"));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lowered = synth_ir::lower(&parsed.ast.unwrap(), &registry, &format!("{label}.synth"));
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let tmp = tempdir(label);
    let result = synth_kicad::export(&lowered.board.unwrap(), &tmp).expect("export");
    fs::read_to_string(&result.pcb_path).expect(".kicad_pcb written")
}

fn squashed(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" )", ")")
}

#[test]
fn declared_stackup_is_written_inside_setup_with_real_values() {
    let pcb = squashed(&export_pcb("stackup_four", 4, FOUR_LAYER_STACKUP));

    let setup = pcb.find("(setup").expect("setup block");
    let stackup = pcb.find("(stackup").expect("stackup section");
    assert!(stackup > setup, "stackup must live inside setup");

    let expected = [
        "(layer \"F.Cu\" (type \"copper\") (thickness 0.035))",
        "(layer \"dielectric 1\" (type \"dielectric\") (thickness 0.2104) (material \"FR4\") \
         (epsilon_r 4.4))",
        "(layer \"In1.Cu\" (type \"copper\") (thickness 0.03556))",
        "(layer \"dielectric 2\" (type \"dielectric\") (thickness 1.065) (epsilon_r 4.6))",
        "(layer \"In2.Cu\" (type \"copper\") (thickness 0.0152))",
        "(layer \"dielectric 3\" (type \"dielectric\") (thickness 0.2104) (material \"FR4\") \
         (epsilon_r 4.4))",
        "(layer \"B.Cu\" (type \"copper\") (thickness 0.035))",
    ];
    let mut from = stackup;
    for layer in expected {
        let at = pcb[from..]
            .find(layer)
            .unwrap_or_else(|| panic!("{layer} missing or out of order in {}", &pcb[stackup..]));
        from += at + layer.len();
    }
}

#[test]
fn board_thickness_is_the_stack_total_only_when_a_stackup_is_declared() {
    let with = squashed(&export_pcb("stackup_total", 4, FOUR_LAYER_STACKUP));
    assert!(with.contains("(general (thickness 1.60656)"), "{with}");
}

#[test]
fn a_design_without_a_stackup_keeps_the_fixed_thickness_and_no_stackup() {
    let pcb = squashed(&export_pcb("stackup_none", 4, ""));
    assert!(!pcb.contains("(stackup"));
    assert!(pcb.contains("(general (thickness 1.6)"), "{pcb}");
}
