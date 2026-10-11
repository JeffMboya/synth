//! The automatic placer must not leave wide empty bands between courtyards.

fn load_board(path: &str) -> synth_ir::Board {
    let source = std::fs::read_to_string(path).expect("read fixture");
    let file = path.to_string();
    let parse = synth_parser::parse(&source, file.clone());
    let ast = parse.ast.as_ref().expect("parse");
    let registry_dir = std::path::Path::new("../..").join("registry").join("parts");
    let registry = synth_registry::load_dir(&registry_dir).expect("registry");
    let loader = synth_ir::FsImportLoader {
        root: std::path::PathBuf::from("../.."),
    };
    let resolved = synth_ir::resolve_imports(ast, &loader, &file);
    synth_ir::lower(&resolved.program, &registry, &file)
        .board
        .expect("board")
}

/// `ref_usb_cdc` was 54.6 x 53.0 mm (2895 mm2) with a 3.0 mm gap between
/// courtyards and 1663 mm2 with 2.0 mm. The bound sits between the two.
#[test]
fn usb_cdc_outline_stays_compact() {
    let board = load_board("../../fixtures/kicad-reference/ref_usb_cdc.synth");
    let placement = synth_place::place(&board).expect("place");
    let area_mm2 = synth_geometry::nm_to_mm(placement.board_outline.width_nm())
        * synth_geometry::nm_to_mm(placement.board_outline.height_nm());
    assert!(
        area_mm2 < 2000.0,
        "outline is {area_mm2:.0} mm2, expected under 2000"
    );
}
