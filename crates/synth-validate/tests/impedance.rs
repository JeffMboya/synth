// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

use synth_diagnostics::{Diagnostic, Severity};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn diagnostics(fixture: &str) -> Vec<Diagnostic> {
    let path = workspace_root()
        .join("fixtures")
        .join("erc")
        .join(format!("{fixture}.synth"));
    let src = std::fs::read_to_string(path).expect("fixture must exist");
    let filename = format!("{fixture}.synth");
    let parse = synth_parser::parse(&src, filename.clone());
    let ast = parse.ast.as_ref().expect("source must parse");
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("registry must load");
    let lowered = synth_ir::lower(ast, &registry, &filename);
    synth_validate::run_erc(lowered.board.as_ref().expect("board must lower"), &filename)
}

fn with_code<'a>(diags: &'a [Diagnostic], code: &str) -> Vec<&'a Diagnostic> {
    diags.iter().filter(|d| d.code == code).collect()
}

fn says_estimate(d: &Diagnostic) {
    assert!(
        d.title.contains("estimate") && d.title.contains("expert analysis"),
        "{}",
        d.title
    );
}

#[test]
fn a_width_that_gives_the_target_or_no_declared_width_is_not_flagged() {
    for fixture in [
        "pass__impedance_width_ok",
        "pass__impedance_derived",
        "pass__impedance_class_without_width",
    ] {
        let diags = diagnostics(fixture);
        assert!(
            diags.iter().all(|d| !d.code.contains("IMPEDANCE")),
            "{fixture}: {diags:?}"
        );
    }
}

#[test]
fn a_width_over_twenty_percent_off_is_an_error_that_says_estimate() {
    let diags = diagnostics("E-SYNTH-IMPEDANCE-001__width_error");
    let flagged = with_code(&diags, "E-SYNTH-IMPEDANCE-001");
    assert_eq!(flagged.len(), 1, "{diags:?}");
    assert_eq!(flagged[0].severity, Severity::Error);
    says_estimate(flagged[0]);
}

#[test]
fn a_width_ten_to_twenty_percent_off_is_a_warning() {
    let diags = diagnostics("E-SYNTH-IMPEDANCE-001__width_warning");
    let flagged = with_code(&diags, "E-SYNTH-IMPEDANCE-001");
    assert_eq!(flagged.len(), 1, "{diags:?}");
    assert_eq!(flagged[0].severity, Severity::Warning);
    says_estimate(flagged[0]);
}

#[test]
fn an_unreachable_target_is_an_error_that_says_estimate() {
    let diags = diagnostics("E-SYNTH-IMPEDANCE-002__unreachable_target");
    let flagged = with_code(&diags, "E-SYNTH-IMPEDANCE-002");
    assert_eq!(flagged.len(), 1, "{diags:?}");
    assert_eq!(flagged[0].severity, Severity::Error);
    says_estimate(flagged[0]);
    assert!(
        flagged[0]
            .title
            .contains("outside the range this estimate covers"),
        "{}",
        flagged[0].title
    );
    assert!(
        with_code(&diags, "E-SYNTH-IMPEDANCE-001").is_empty(),
        "{diags:?}"
    );
}

#[test]
fn a_net_that_may_use_inner_layers_is_labelled_not_verified() {
    let diags = diagnostics("W-SYNTH-IMPEDANCE-001__inner_layers");
    let flagged = with_code(&diags, "W-SYNTH-IMPEDANCE-001");
    assert_eq!(flagged.len(), 1, "{diags:?}");
    assert_eq!(flagged[0].severity, Severity::Warning);
    says_estimate(flagged[0]);
    assert!(flagged[0]
        .found
        .as_deref()
        .unwrap()
        .contains("not verified"));
}

#[test]
fn a_board_without_a_stackup_is_not_checked() {
    let diags = diagnostics("pass__rf_003_with_impedance");
    assert!(
        diags.iter().all(|d| !d.code.contains("IMPEDANCE")),
        "{diags:?}"
    );
}
