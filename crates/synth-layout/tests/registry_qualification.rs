// SPDX-License-Identifier: Apache-2.0

use std::path::{Path, PathBuf};

use synth_layout::qualify_facts::InstalledKicad;
use synth_registry::qualify::{qualify_part, CheckStatus, FindingLevel, PartQualification};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

fn fixture_root() -> PathBuf {
    workspace_root().join("fixtures").join("registry-qualify")
}

fn load(dir: &str, id: &str) -> synth_registry::Part {
    let registry = synth_registry::load_dir(&fixture_root().join(dir))
        .unwrap_or_else(|e| panic!("load {dir}: {e}"));
    registry
        .lookup(id)
        .unwrap_or_else(|| panic!("no part `{id}` in {dir}"))
        .clone()
}

fn qualify_mutation(id: &str) -> PartQualification {
    let facts = InstalledKicad;
    qualify_part(&load(&format!("mutations/{id}"), id), &facts, &facts)
}

fn codes(q: &PartQualification) -> Vec<String> {
    q.findings().map(|f| f.code.clone()).collect()
}

fn qualify(dir: &str, id: &str) -> PartQualification {
    let facts = InstalledKicad;
    qualify_part(&load(dir, id), &facts, &facts)
}

/// One test, not many: the footprint loader caches its search root in a
/// `OnceLock`, so the fixture directory has to be in place before the first
/// lookup in this process and cannot be changed afterwards.
#[test]
fn fixtures_qualify_and_every_mutation_is_caught() {
    std::env::set_var("KICAD_FOOTPRINT_DIR", fixture_root().join("footprints"));
    representative_parts_qualify();
    every_mutation_is_caught();
    structural_classification_matches_the_export_gate();
}

fn representative_parts_qualify() {
    for id in [
        "qual_mcu_qfn",
        "qual_regulator",
        "qual_connector",
        "qual_rf_module",
        "qual_bga",
    ] {
        let q = qualify("parts", id);
        let structural: Vec<&str> = q.structural_defects().map(|f| f.code.as_str()).collect();
        assert!(
            structural.is_empty(),
            "{id} should be structurally sound, got {structural:?}\n{:#?}",
            q.checks
        );
        assert_eq!(
            q.status,
            CheckStatus::Pass,
            "{id} should fully qualify\n{:#?}",
            q.checks
        );
    }
}

fn every_mutation_is_caught() {
    let expectations: &[(&str, &str, FindingLevel)] = &[
        (
            "mut_missing_pad",
            "E-SYNTH-QUAL-001",
            FindingLevel::Blocking,
        ),
        (
            "mut_undeclared_exposed_pad",
            "E-SYNTH-QUAL-002",
            FindingLevel::Blocking,
        ),
        (
            "mut_wrong_units",
            "E-SYNTH-QUAL-006",
            FindingLevel::Blocking,
        ),
        (
            "mut_mirrored_footprint",
            "E-SYNTH-QUAL-012",
            FindingLevel::Blocking,
        ),
        (
            "mut_stale_provenance",
            "E-SYNTH-QUAL-015",
            FindingLevel::Blocking,
        ),
        ("mut_unreviewed", "E-SYNTH-QUAL-008", FindingLevel::Blocking),
    ];

    for (id, code, level) in expectations {
        let q = qualify_mutation(id);
        let found = codes(&q);
        assert!(
            found.iter().any(|c| c == code),
            "{id} must report {code}, got {found:?}\n{:#?}",
            q.checks
        );
        let finding = q
            .findings()
            .find(|f| f.code == *code)
            .expect("just asserted present");
        assert_eq!(finding.level, *level, "{id}: {code} level");
        assert_ne!(
            q.status,
            CheckStatus::Pass,
            "{id} must not qualify\n{:#?}",
            q.checks
        );
        assert!(
            !q.is_fabrication_safe(),
            "{id} must not be fabrication safe"
        );
    }

    let wrong_units = qualify_mutation("mut_wrong_units");
    assert!(
        wrong_units
            .findings()
            .any(|f| f.code == "E-SYNTH-QUAL-006" && f.found.contains("recorded as mils")),
        "the unit mistake should be named, not just flagged:\n{:#?}",
        wrong_units.checks
    );
}

fn structural_classification_matches_the_export_gate() {
    let connector = qualify("parts", "qual_connector");
    assert!(
        !codes(&connector).iter().any(|c| c == "E-SYNTH-QUAL-002"),
        "the NPTH mounting hole must not read as an undeclared pad:\n{:#?}",
        connector.checks
    );

    for (id, structural) in [
        ("mut_missing_pad", true),
        ("mut_undeclared_exposed_pad", true),
        ("mut_mirrored_footprint", true),
        ("mut_unreviewed", false),
        ("mut_stale_provenance", false),
    ] {
        let q = qualify_mutation(id);
        assert_eq!(
            q.has_structural_defect(),
            structural,
            "{id} structural classification drives the export gate\n{:#?}",
            q.structural_defects().collect::<Vec<_>>()
        );
    }
}

/// A pin map that reuses one pad number never reaches the qualification
/// engine: the registry loader rejects it. Asserting that here keeps the
/// mutation corpus honest about which layer catches what.
#[test]
fn a_duplicated_pad_number_is_rejected_at_load() {
    let err = synth_registry::load_dir(&fixture_root().join("rejected-at-load"))
        .expect_err("a duplicated pad number must not load");
    let text = err.to_string();
    assert!(
        text.contains("duplicate pin number"),
        "expected a duplicate-pin-number refusal, got: {text}"
    );
}
