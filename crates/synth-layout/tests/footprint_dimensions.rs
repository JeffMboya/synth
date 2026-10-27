// SPDX-License-Identifier: Apache-2.0

//! The footprint size a part declares, derived from a footprint's text. The
//! fixtures are checked in, so none of this needs KiCad installed.

use synth_layout::kicad_footprint_loader::{
    courtyard_extent_from_text, derive_footprint_dimensions, derive_footprint_dimensions_from_text,
    pads_extent_from_text,
};

const MARGIN: f64 = 0.25;

const SOT23: &str = include_str!("fixtures/footprints/sot23_like.kicad_mod");
const TO92_MIXED: &str = include_str!("fixtures/footprints/to92_mixed.kicad_mod");
const CIRCLE: &str = include_str!("fixtures/footprints/circle_courtyard.kicad_mod");
const POLY: &str = include_str!("fixtures/footprints/poly_courtyard.kicad_mod");
const NETTIE: &str = include_str!("fixtures/footprints/nettie_no_courtyard.kicad_mod");
const DEGENERATE: &str = include_str!("fixtures/footprints/degenerate_courtyard.kicad_mod");
const EMPTY: &str = include_str!("fixtures/footprints/empty.kicad_mod");

fn derived(text: &str) -> Result<(f64, f64), String> {
    derive_footprint_dimensions_from_text(text, MARGIN)
}

#[test]
fn the_margin_is_taken_off_the_courtyard_once() {
    // The courtyard is 3.86 x 3.4: the body plus the margin on each side.
    let (x0, y0, x1, y1) = courtyard_extent_from_text(SOT23).expect("a courtyard");
    assert!((x1 - x0 - 3.86).abs() < 1e-9 && (y1 - y0 - 3.4).abs() < 1e-9);
    assert_eq!(derived(SOT23), Ok((3.36, 2.9)));
}

#[test]
fn a_courtyard_drawn_with_a_line_and_an_arc_is_measured_whole() {
    // Its only line is horizontal, so reading lines alone gives a height of zero;
    // the arc carries the other 3.5 mm.
    let (x0, y0, x1, y1) = courtyard_extent_from_text(TO92_MIXED).expect("a courtyard");
    assert!((x1 - x0 - 4.0).abs() < 1e-9, "{x0} {x1}");
    assert!((y1 - y0 - 3.5).abs() < 1e-9, "{y0} {y1}");
    assert_eq!(derived(TO92_MIXED), Ok((3.5, 3.0)));
}

#[test]
fn a_circle_and_a_polygon_courtyard_are_measured() {
    assert_eq!(
        derived(CIRCLE),
        Ok((2.5, 2.5)),
        "a 1.5 mm radius is 3 mm across"
    );
    assert_eq!(
        derived(POLY),
        Ok((4.5, 2.5)),
        "5 x 3 less the margin on each side"
    );
}

#[test]
fn a_footprint_without_a_courtyard_takes_its_pads_extent() {
    assert_eq!(courtyard_extent_from_text(NETTIE), None);
    let (w, h) = pads_extent_from_text(NETTIE).expect("pads");
    assert!((w - 1.5).abs() < 1e-9 && (h - 0.5).abs() < 1e-9);
    assert_eq!(derived(NETTIE), Ok((1.5, 0.5)));
}

#[test]
fn a_degenerate_courtyard_falls_back_to_the_pads_not_to_zero() {
    // One line: a height of zero, which the registry refuses to load.
    assert_eq!(
        derived(DEGENERATE),
        Ok((3.0, 1.0)),
        "two 1 mm pads 2 mm apart"
    );
}

#[test]
fn a_footprint_with_nothing_to_measure_is_refused_with_a_reason() {
    let why = derived(EMPTY).expect_err("nothing to measure");
    assert!(
        why.contains("no courtyard") && why.contains("no pads"),
        "{why}"
    );
}

#[test]
fn an_unknown_footprint_is_refused_and_named() {
    let why = derive_footprint_dimensions("No_Such_Library:No_Such_Footprint", MARGIN)
        .expect_err("it cannot be read");
    assert!(why.contains("No_Such_Library:No_Such_Footprint"), "{why}");
    assert!(why.contains("spelling"), "{why}");
}
