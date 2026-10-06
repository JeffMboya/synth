// SPDX-License-Identifier: Apache-2.0

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
        "synth-kicad-impedance-{label}-{}",
        std::process::id()
    ));
    if p.exists() {
        let _ = fs::remove_dir_all(&p);
    }
    p
}

const STACKUP: &str = r#"
  stackup {
    copper 0.035mm
    insulator 0.2104mm er 4.4 material "FR4"
    copper 0.0152mm
    insulator 1.065mm er 4.6
    copper 0.0152mm
    insulator 0.2104mm er 4.4 material "FR4"
    copper 0.035mm
  }
"#;

fn export_pcb(label: &str, impedance: &str, extra: &str) -> String {
    let pair = format!("  diff_pair r1_p1 r2_p1 {{ impedance {impedance} }}\n");
    export_board(label, "SIG", &pair, extra)
}

fn export_board(label: &str, net: &str, pair: &str, extra: &str) -> String {
    let source = format!(
        r#"board "{label}" {{
  layers 4
  component R1: resistor "r_generic_0603" value "10k"
  component R2: resistor "r_generic_0603" value "10k"
  connect R1.p1 -> R2.p1 as "{net}"
  connect R1.p2 -> R2.p2
{pair}{extra}}}
"#
    );
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

fn classes_holding_sig(pcb: &str) -> Vec<(String, f64)> {
    let pcb = squashed(pcb);
    pcb.split("(net_class ")
        .skip(1)
        .filter(|block| block.contains("(add_net \"SIG\")"))
        .map(|block| {
            let name = block.split('"').nth(1).unwrap().to_string();
            let width = block
                .split("(trace_width ")
                .nth(1)
                .and_then(|rest| rest.split(')').next())
                .unwrap()
                .parse()
                .unwrap();
            (name, width)
        })
        .collect()
}

#[test]
fn two_targets_on_one_stackup_export_different_net_classes() {
    let at_50 = classes_holding_sig(&export_pcb("z50", "50ohm", STACKUP));
    let at_75 = classes_holding_sig(&export_pcb("z75", "75ohm", STACKUP));

    assert_eq!(at_50.len(), 1, "{at_50:?}");
    assert_eq!(at_75.len(), 1, "{at_75:?}");
    assert_ne!(at_50[0].0, at_75[0].0);
    assert!((at_50[0].1 - 0.3498).abs() < 0.002, "{at_50:?}");
    assert!((at_75[0].1 - 0.1531).abs() < 0.002, "{at_75:?}");
}

#[test]
fn the_fixed_50_ohm_rf_class_is_gone() {
    for pcb in [
        export_pcb("fixed_gone", "50ohm", STACKUP),
        export_board("fixed_gone_by_name", "MAIN_ANT", "", STACKUP),
    ] {
        assert!(!pcb.contains("Controlled 50 ohm RF"));
        assert!(!pcb.contains("RF_50"));
    }
}

#[test]
fn without_a_stackup_no_controlled_class_is_invented() {
    let pcb = export_pcb("no_stackup", "50ohm", "");
    let classes = classes_holding_sig(&pcb);
    assert_eq!(classes.len(), 1, "{classes:?}");
    assert_eq!(classes[0], ("Default".to_string(), 0.127));
}

#[test]
fn an_unreachable_target_gets_no_controlled_class() {
    let classes = classes_holding_sig(&export_pcb("unreachable", "5ohm", STACKUP));
    assert_eq!(classes, [("Default".to_string(), 0.127)]);
}

#[test]
fn a_declared_width_stays_in_the_declared_class() {
    let extra = format!(
        "  netclass \"RF\" {{ trace_width 0.3mm }}\n  net \"SIG\" class \"RF\" {{ R1.p1, R2.p1 }}\n{STACKUP}"
    );
    let pcb = export_pcb("declared", "50ohm", &extra);
    let classes = classes_holding_sig(&pcb);
    assert_eq!(classes, [("RF".to_string(), 0.3)]);
}

#[test]
fn a_class_without_a_width_still_gets_the_derived_width() {
    let extra = format!(
        "  netclass \"RF\" {{ clearance 0.2mm }}\n  net \"SIG\" class \"RF\" {{ R1.p1, R2.p1 }}\n{STACKUP}"
    );
    let classes = classes_holding_sig(&export_pcb("class_no_width", "50ohm", &extra));
    assert_eq!(classes.len(), 1, "{classes:?}");
    assert_eq!(classes[0].0, "Z0_50OHM");
    assert!((classes[0].1 - 0.3498).abs() < 0.002, "{classes:?}");
}
