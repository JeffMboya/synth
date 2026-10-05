// SPDX-License-Identifier: Apache-2.0

use std::process::Command;

const SYNTH: &str = env!("CARGO_BIN_EXE_synth");

#[test]
fn capability_list_documents_the_stackup_block() {
    let out = Command::new(SYNTH)
        .args(["capability", "list", "--json"])
        .output()
        .expect("run synth");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let descriptor: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");

    let statements = descriptor["language"]["statements"].as_array().unwrap();
    assert!(statements.iter().any(|s| s == "stackup"), "{statements:?}");

    let stackup = &descriptor["geometry_and_constraints"]["stackup"];
    assert_eq!(stackup["statement"], "stackup");
    assert_eq!(
        stackup["entries"],
        serde_json::json!(["copper", "insulator"])
    );
    assert_eq!(stackup["insulator"]["required"], serde_json::json!(["er"]));
}
