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
    let result = export_files(label, net, pair, extra);
    fs::read_to_string(&result.pcb_path).expect(".kicad_pcb written")
}

fn export_files(label: &str, net: &str, pair: &str, extra: &str) -> synth_kicad::ExportResult {
    let source = format!(
        r#"board "{label}" {{
  layers 4
  component R1: resistor "r_generic_0603" value "10k"
  component R2: resistor "r_generic_0603" value "10k"
  connect R1.p1 -> R2.p1 as "{net}"
  connect R1.p2 -> R2.p2 as "{net}_N"
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
    synth_kicad::export(&lowered.board.unwrap(), &tmp).expect("export")
}

fn pair_decl(impedance: &str) -> String {
    format!("  diff_pair USB USB_N {{ impedance {impedance} }}\n")
}

fn export_pair(label: &str, impedance: &str, extra: &str) -> String {
    export_board(label, "USB", &pair_decl(impedance), extra)
}

fn squashed(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" )", ")")
}

fn blocks_holding(pcb: &str, net: &str) -> Vec<String> {
    squashed(pcb)
        .split("(net_class ")
        .skip(1)
        .filter(|block| block.contains(&format!("(add_net \"{net}\")")))
        .map(str::to_string)
        .collect()
}

fn name_of(block: &str) -> String {
    block.split('"').nth(1).unwrap().to_string()
}

fn field(block: &str, key: &str) -> Option<f64> {
    block
        .split(&format!("({key} "))
        .nth(1)
        .and_then(|rest| rest.split(')').next())
        .map(|value| value.parse().unwrap())
}

fn classes_holding_sig(pcb: &str) -> Vec<(String, f64)> {
    blocks_holding(pcb, "SIG")
        .iter()
        .map(|block| (name_of(block), field(block, "trace_width").unwrap()))
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

fn declared_pair(width_mm: f64, clearance: &str) -> String {
    format!(
        "  netclass \"HS\" {{ trace_width {width_mm}mm {clearance} }}\n  net \"USB\" class \"HS\" {{ R1.p1, R2.p1 }}\n  net \"USB_N\" class \"HS\" {{ R1.p2, R2.p2 }}\n{STACKUP}"
    )
}

fn assert_class(pcb: &str, net: &str, name: &str, fields: &[(&str, f64)]) {
    let blocks = blocks_holding(pcb, net);
    assert_eq!(blocks.len(), 1, "{blocks:?}");
    assert_eq!(name_of(&blocks[0]), name);
    for (key, want) in fields {
        let got = field(&blocks[0], key).unwrap_or_else(|| panic!("{key} missing: {blocks:?}"));
        assert!((got - want).abs() < 0.002, "{key}: {got} != {want}");
    }
}

#[test]
fn a_90_ohm_and_a_5_ohm_pair_export_different_net_classes() {
    let at_90 = export_pair("pair90", "90ohm", STACKUP);
    let at_5 = export_pair("pair5", "5ohm", STACKUP);

    let derived = [
        ("trace_width", 0.2421),
        ("diff_pair_width", 0.2421),
        ("diff_pair_gap", 0.127),
        ("clearance", 0.127),
    ];
    assert_class(&at_90, "USB", "ZDIFF_90OHM", &derived);
    assert_class(&at_90, "USB_N", "ZDIFF_90OHM", &derived);
    assert_class(&at_5, "USB", "Default", &[]);
    assert_class(&at_5, "USB_N", "Default", &[]);
}

#[test]
fn a_pair_without_a_stackup_gets_no_controlled_class() {
    let pcb = export_pair("pair_no_stackup", "90ohm", "");
    assert_class(&pcb, "USB", "Default", &[]);
}

#[test]
fn accepted_and_warned_pairs_keep_their_declared_class_and_fields() {
    for (label, width_mm) in [("pair_accepted", 0.242), ("pair_warned", 0.17)] {
        let pcb = export_pair(
            label,
            "90ohm",
            &declared_pair(width_mm, "clearance 0.127mm"),
        );
        let fields = [
            ("trace_width", width_mm),
            ("diff_pair_width", width_mm),
            ("diff_pair_gap", 0.127),
            ("clearance", 0.127),
        ];
        assert_class(&pcb, "USB", "HS", &fields);
        assert_class(&pcb, "USB_N", "HS", &fields);
        assert!(!pcb.contains("ZDIFF"));
    }
}

#[test]
fn half_declared_and_mismatched_pairs_derive_nothing_and_carry_no_pair_fields() {
    let half = format!(
        "  netclass \"HS\" {{ trace_width 0.242mm clearance 0.127mm }}\n  net \"USB\" class \"HS\" {{ R1.p1, R2.p1 }}\n{STACKUP}"
    );
    let mismatched = format!(
        "  netclass \"HS\" {{ trace_width 0.242mm clearance 0.127mm }}\n  netclass \"HS2\" {{ trace_width 0.17mm clearance 0.127mm }}\n  net \"USB\" class \"HS\" {{ R1.p1, R2.p1 }}\n  net \"USB_N\" class \"HS2\" {{ R1.p2, R2.p2 }}\n{STACKUP}"
    );
    for (label, extra, classes) in [
        ("pair_half", half, [("USB", "HS"), ("USB_N", "Default")]),
        (
            "pair_mismatch",
            mismatched,
            [("USB", "HS"), ("USB_N", "HS2")],
        ),
    ] {
        let result = export_files(label, "USB", &pair_decl("90ohm"), &extra);
        let pcb = fs::read_to_string(&result.pcb_path).unwrap();
        let pro: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&result.project_path).unwrap()).unwrap();

        assert!(!pcb.contains("ZDIFF"), "{label}");
        assert!(!pcb.contains("diff_pair_width"), "{label}");
        assert!(!pcb.contains("diff_pair_gap"), "{label}");
        for (net, class) in classes {
            assert_class(&pcb, net, class, &[]);
            let in_pro = pro_class(&pro, class);
            assert_eq!(in_pro["diff_pair_width"], 0.2, "{label} {class}");
            assert_eq!(in_pro["diff_pair_gap"], 0.25, "{label} {class}");
        }
    }
}

fn pro_class(pro: &serde_json::Value, name: &str) -> serde_json::Value {
    pro["net_settings"]["classes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|class| class["name"] == name)
        .unwrap_or_else(|| panic!("{name} missing from {pro}"))
        .clone()
}

#[test]
fn a_declared_pair_class_reaches_kicad_with_the_checked_width_and_gap() {
    for (label, clearance, gap) in [
        ("pro_gap", "clearance 0.2mm", 0.2),
        ("pro_no_gap", "", 0.127),
    ] {
        let extra = format!(
            "  netclass \"PWR\" {{ trace_width 0.5mm }}\n{}",
            declared_pair(0.242, clearance)
        );
        let result = export_files(label, "USB", &pair_decl("90ohm"), &extra);
        let pcb = fs::read_to_string(&result.pcb_path).unwrap();
        let pro: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&result.project_path).unwrap()).unwrap();

        assert_class(
            &pcb,
            "USB",
            "HS",
            &[("diff_pair_width", 0.242), ("diff_pair_gap", gap)],
        );
        let hs = pro_class(&pro, "HS");
        assert_eq!(hs["diff_pair_width"], 0.242);
        assert_eq!(hs["diff_pair_gap"], gap);
        for untouched in ["PWR", "Default"] {
            let class = pro_class(&pro, untouched);
            assert_eq!(class["diff_pair_width"], 0.2, "{untouched}");
            assert_eq!(class["diff_pair_gap"], 0.25, "{untouched}");
        }
    }
}

const SECOND_NETS: &str = "  component R3: resistor \"r_generic_0603\" value \"10k\"\n  component R4: resistor \"r_generic_0603\" value \"10k\"\n  connect R3.p1 -> R4.p1 as \"B\"\n  connect R3.p2 -> R4.p2 as \"B_N\"\n";

fn export_with_second_nets(label: &str, extra: &str) -> (String, serde_json::Value) {
    let extra = format!("{SECOND_NETS}{extra}");
    let result = export_files(label, "USB", &pair_decl("90ohm"), &extra);
    let pro = fs::read_to_string(&result.project_path).unwrap();
    (
        fs::read_to_string(&result.pcb_path).unwrap(),
        serde_json::from_str(&pro).unwrap(),
    )
}

#[test]
fn a_class_carrying_a_valid_and_a_mismatched_pair_emits_no_pair_fields() {
    let extra = format!(
        "  diff_pair B B_N {{ impedance 90ohm }}\n  netclass \"HS\" {{ trace_width 0.242mm clearance 0.127mm }}\n  netclass \"HS2\" {{ trace_width 0.17mm clearance 0.127mm }}\n  net \"USB\" class \"HS\" {{ R1.p1, R2.p1 }}\n  net \"USB_N\" class \"HS\" {{ R1.p2, R2.p2 }}\n  net \"B\" class \"HS\" {{ R3.p1, R4.p1 }}\n  net \"B_N\" class \"HS2\" {{ R3.p2, R4.p2 }}\n{STACKUP}"
    );
    let (pcb, pro) = export_with_second_nets("pair_valid_and_mismatched", &extra);

    assert!(!pcb.contains("diff_pair_width"));
    assert!(!pcb.contains("diff_pair_gap"));
    for class in ["HS", "HS2"] {
        let in_pro = pro_class(&pro, class);
        assert_eq!(in_pro["diff_pair_width"], 0.2, "{class}");
        assert_eq!(in_pro["diff_pair_gap"], 0.25, "{class}");
    }
}

#[test]
fn an_unrelated_net_sharing_the_class_does_not_remove_a_valid_pairs_fields() {
    let extra = format!(
        "  netclass \"HS\" {{ trace_width 0.242mm clearance 0.127mm }}\n  net \"USB\" class \"HS\" {{ R1.p1, R2.p1 }}\n  net \"USB_N\" class \"HS\" {{ R1.p2, R2.p2 }}\n  net \"B\" class \"HS\" {{ R3.p1, R4.p1 }}\n{STACKUP}"
    );
    let (pcb, pro) = export_with_second_nets("pair_with_unrelated_net", &extra);

    let fields = [("diff_pair_width", 0.242), ("diff_pair_gap", 0.127)];
    assert_class(&pcb, "USB", "HS", &fields);
    assert_class(&pcb, "B", "HS", &fields);
    assert_eq!(pro_class(&pro, "HS")["diff_pair_width"], 0.242);
}
