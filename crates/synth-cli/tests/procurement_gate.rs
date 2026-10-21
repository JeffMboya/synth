use std::{fs, process::Command};

#[test]
fn invalid_designs_never_replace_a_procurement_lock() {
    for source in [
        "board \"empty\" { layers 2 }",
        "board \"broken\" { component U1: mcu \"not_a_registry_part\" }",
        "board \"broken\" { layers invalid }",
        "import \"missing.synth\"\nboard \"broken\" { layers 2 }",
    ] {
        let directory = std::env::temp_dir().join(format!(
            "synth-procurement-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let input = directory.join("input.synth");
        let output = directory.join("procurement.json");
        fs::write(&input, source).unwrap();
        fs::write(&output, "previous reviewed lock").unwrap();
        let result = Command::new(env!("CARGO_BIN_EXE_synth"))
            .arg("procurement-lock")
            .arg(&input)
            .arg("--out")
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            !result.status.success(),
            "invalid source was locked: {source}"
        );
        assert_eq!(
            fs::read_to_string(&output).unwrap(),
            "previous reviewed lock"
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
