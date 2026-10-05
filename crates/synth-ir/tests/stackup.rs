// SPDX-License-Identifier: Apache-2.0

//! `stackup { … }` lowering: the ordered layers land on the board as typed
//! lengths and dielectric constants.

use synth_ir::{DielectricConstant, Length, StackupLayer};

fn lower(src: &str) -> (Option<synth_ir::Board>, Vec<String>) {
    let parse = synth_parser::parse(src, "stackup.synth");
    assert!(!parse.has_errors(), "{:?}", parse.diagnostics);
    let ast = parse.ast.expect("clean parse yields an ast");
    let lowered = synth_ir::lower(&ast, &synth_registry::Registry::default(), "stackup.synth");
    (
        lowered.board,
        lowered.diagnostics.iter().map(|d| d.code.clone()).collect(),
    )
}

const FOUR_LAYER: &str = r#"board "b" {
    layers 4
    stackup {
        copper 0.035mm
        insulator 0.2104mm er 4.4 material "FR4"
        copper 1.4mil
        insulator 1.065mm er 4.6
        copper 0.0152mm
        insulator 0.2104mm er 4.4 material "FR4"
        copper 0.035mm
    }
}"#;

#[test]
fn stackup_lowers_into_ordered_typed_layers() {
    let (board, codes) = lower(FOUR_LAYER);
    assert!(codes.is_empty(), "{codes:?}");
    let stackup = board.unwrap().stackup.expect("stackup declared");
    assert_eq!(stackup.layers.len(), 7);
    assert_eq!(stackup.copper_count(), 4);
    assert!(matches!(
        &stackup.layers[1],
        StackupLayer::Insulator { thickness, er, material, .. }
            if *thickness == Length(210_400)
                && *er == DielectricConstant(4_400_000)
                && material.as_deref() == Some("FR4")
    ));
    assert!(matches!(
        &stackup.layers[2],
        StackupLayer::Copper { thickness, .. } if *thickness == Length(35_560)
    ));
    assert!(matches!(
        &stackup.layers[3],
        StackupLayer::Insulator { material: None, .. }
    ));
    assert_eq!(stackup.total_thickness(), Length(1_606_560));
}

#[test]
fn a_design_without_a_stackup_has_none() {
    let (board, codes) = lower("board \"b\" {\n layers 2\n}");
    assert!(codes.is_empty(), "{codes:?}");
    assert!(board.unwrap().stackup.is_none());
}

#[test]
fn a_thickness_that_is_not_a_length_is_a_unit_error() {
    let src = r#"board "b" {
        layers 2
        stackup { copper 0.035ohm insulator 1.5mm er 4.2 copper 0.035mm }
    }"#;
    let (_, codes) = lower(src);
    assert!(codes.contains(&"E-SYNTH-UNIT-001".to_string()), "{codes:?}");
}
