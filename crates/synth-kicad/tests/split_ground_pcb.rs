// SPDX-License-Identifier: Apache-2.0

//! A design that declares separate analog and digital grounds must keep both
//! names in the exported `.kicad_pcb`, and every generated ground zone must
//! carry its own uuid.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const SPLIT_GROUND_SOURCE: &str = r#"board "split_gnd" {
  layers 4
  component C1: capacitor "c_generic_0603" value "100nF"
  component C2: capacitor "c_generic_0603" value "100nF"
  component C3: capacitor "c_generic_0603" value "100nF"
  component C4: capacitor "c_generic_0603" value "100nF"
  component C5: capacitor "c_generic_0603" value "100nF"
  component C6: capacitor "c_generic_0603" value "100nF"
  connect C1.p2 -> C2.p2 as "AGND"
  connect C2.p2 -> C3.p2
  connect C4.p2 -> C5.p2 as "DGND"
  connect C5.p2 -> C6.p2
}
"#;

const SINGLE_GROUND_SOURCE: &str = r#"board "single_gnd" {
  layers 4
  component C1: capacitor "c_generic_0603" value "100nF"
  component C2: capacitor "c_generic_0603" value "100nF"
  component R1: resistor "r_generic_0603" value "10k"
  connect R1.p1 -> C1.p1
  connect C1.p2 -> C2.p2 as "AGND"
  connect R1.p2 -> C2.p1
}
"#;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn export_pcb(source: &str, label: &str) -> String {
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("seed registry must load");
    let parsed = synth_parser::parse(source, "split_gnd.synth");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lowered = synth_ir::lower(&parsed.ast.expect("ast"), &registry, "split_gnd.synth");
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let board = lowered.board.expect("board");

    let out = std::env::temp_dir().join(format!(
        "synth-kicad-split-gnd-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&out);
    let result = synth_kicad::export(&board, &out).expect("export");
    let text = fs::read_to_string(&result.pcb_path).expect(".kicad_pcb written");
    let _ = fs::remove_dir_all(&out);
    text
}

fn quoted_values_after<'a>(text: &'a str, key: &str) -> Vec<&'a str> {
    text.match_indices(key)
        .filter_map(|(at, _)| text[at + key.len()..].split('"').next())
        .collect()
}

fn declared_nets(pcb: &str) -> Vec<&str> {
    pcb.lines()
        .filter_map(|line| line.trim().strip_prefix("(net "))
        .filter_map(|rest| rest.split('"').nth(1))
        .collect()
}

#[test]
fn split_grounds_keep_their_names_and_each_zone_its_own_uuid() {
    let pcb = export_pcb(SPLIT_GROUND_SOURCE, "split");

    let declared = declared_nets(&pcb);
    for name in ["AGND", "DGND"] {
        assert!(declared.contains(&name), "{name} not in {declared:?}");
    }
    assert!(!declared.contains(&"GND"), "AGND was renamed: {declared:?}");

    let zones: Vec<&str> = pcb.split("(zone").skip(1).collect();
    assert_eq!(zones.len(), 8, "expected AGND and DGND zones on 4 layers");
    let uuids: HashSet<&str> = zones
        .iter()
        .filter_map(|zone| quoted_values_after(zone, "(uuid \"").first().copied())
        .collect();
    assert_eq!(uuids.len(), zones.len(), "zones share uuids: {uuids:?}");

    let legacy = [
        "d72fbbe7-0d2e-5ba1-b608-354074f9a01a",
        "39953160-305c-53c9-9c17-4dc783a1353e",
        "c4b942ca-ffc3-5780-a79b-737734c39a95",
        "841f9dfc-bed1-5d60-93e1-7d4bb17f4269",
    ];
    for uuid in legacy {
        assert!(uuids.contains(uuid), "legacy zone uuid {uuid} changed");
    }
}

#[test]
fn a_lone_ground_net_is_still_named_gnd() {
    let pcb = export_pcb(SINGLE_GROUND_SOURCE, "single");

    let declared = declared_nets(&pcb);
    assert!(declared.contains(&"GND"), "{declared:?}");
    assert!(!declared.contains(&"AGND"), "{declared:?}");
}
