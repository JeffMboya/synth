// SPDX-License-Identifier: Apache-2.0

//! `synth capability list` states what Synth cannot do or cannot verify,
//! in both the human table and the JSON descriptor.

use std::process::Command;

fn capability_list(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_synth"))
        .args(["capability", "list"])
        .args(args)
        .output()
        .expect("run synth capability list");
    assert!(
        output.status.success(),
        "capability list failed: {output:?}"
    );
    String::from_utf8(output.stdout).expect("utf-8 output")
}

#[test]
fn json_and_human_output_name_every_unsupported_and_unverified_entry() {
    let descriptor: serde_json::Value =
        serde_json::from_str(&capability_list(&["--json"])).expect("descriptor is JSON");
    let human = capability_list(&[]);

    for (section, heading) in [("unsupported", "UNSUPPORTED"), ("unverified", "UNVERIFIED")] {
        let entries = descriptor[section].as_array().expect("section is an array");
        assert!(!entries.is_empty(), "`{section}` must not be empty");
        assert!(
            human.contains(heading),
            "human output lacks the {heading} heading"
        );
        for entry in entries {
            let id = entry["id"].as_str().expect("entry has an id");
            assert!(
                human.contains(id),
                "human output lacks {section} entry {id}"
            );
        }
    }
}
