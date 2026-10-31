// SPDX-License-Identifier: Apache-2.0

//! Regression tests for pin reconciliation against real KiCad symbol
//! libraries. Only meaningful when KiCad is installed locally; the
//! reconciliation silently no-ops otherwise (so these tests assert on
//! live-KiCad inputs only and are skipped in CI without KiCad).

use std::path::Path;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn build_board(name: &str) -> synth_ir::Board {
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("registry");
    let path = workspace_root().join("fixtures").join("kicad-reference");
    let filename = format!("{name}.synth");
    let src = std::fs::read_to_string(path.join(&filename)).unwrap();
    let parsed = synth_parser::parse(&src, filename.clone());
    let ast = parsed.ast.expect("ast");
    synth_ir::lower(&ast, &registry, &filename)
        .board
        .expect("board")
}

#[test]
fn undriven_nets_ref_usb_cdc() {
    // ref_usb_cdc's +3V3 rail is driven by U2 (AMS1117) vout, a
    // power_out, so it must NOT receive an ERC power driver flag.
    let board = build_board("ref_usb_cdc");
    let layout = synth_layout::layout(&board);
    let placements: std::collections::HashMap<_, _> =
        layout.components.iter().map(|p| (p.id, p)).collect();
    let drivers = synth_kicad::pin_reconcile::undriven_power_nets(&board, &placements);
    for d in &drivers {
        let comp = board.component(d.anchor_component).unwrap();
        assert_ne!(
            comp.refdes, "U2",
            "ref_usb_cdc +3V3 rail (driven by U2.vout) must not receive a PWR_FLAG"
        );
    }
}

#[test]
fn reconcile_fans_out_rp2040_power_legs() {
    if synth_layout::kicad_lib_loader::physical_pins("MCU_RaspberryPi_RP2040:RP2040").is_none() {
        eprintln!("KiCad symbol library not installed locally; skipping pin reconciliation test");
        return;
    }

    // secure_tracker's RP2040 has undeclared physical power legs
    // (ADC_AVDD 43, VREG_VIN 44, USB_VDD 48) that must be fanned onto
    // the rail, and unused GPIOs that must be no-connect.
    let board = build_board("secure_tracker");
    let mut saw_fanout = false;
    for reconciled in synth_kicad::pin_reconcile::reconcile_all(&board) {
        saw_fanout |= !reconciled.power_legs.is_empty();
    }
    assert!(
        saw_fanout,
        "RP2040 power legs should be fanned onto the rail"
    );
}

#[test]
fn usb_uart_vio_rail_gets_a_power_flag() {
    // VIO is fed only through the J_VIO jumper, whose pins are passive, so
    // no pin on the rail is a power output and KiCad ERC would report
    // `power_pin_not_driven` without a flag.
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("registry");
    let filename = "usb_uart.synth";
    let src = std::fs::read_to_string(
        workspace_root()
            .join("qualification")
            .join("class-b")
            .join("usb-uart")
            .join(filename),
    )
    .unwrap();
    let ast = synth_parser::parse(&src, filename.to_string())
        .ast
        .expect("ast");
    let board = synth_ir::lower(&ast, &registry, filename)
        .board
        .expect("board");
    let layout = synth_layout::layout(&board);
    let placements: std::collections::HashMap<_, _> =
        layout.components.iter().map(|p| (p.id, p)).collect();
    let drivers = synth_kicad::pin_reconcile::undriven_power_nets(&board, &placements);
    let vio = drivers
        .iter()
        .find(|d| board.net(d.net).unwrap().name == "VIO")
        .expect("VIO is flagged");
    // U1 (CH342F) has no stock symbol, so it cannot anchor the flag stub.
    let anchor = board.component(vio.anchor_component).unwrap();
    assert!(
        anchor.part.as_ref().unwrap().kicad_symbol.is_some(),
        "anchored on {}",
        anchor.refdes
    );
}

#[test]
fn a_rail_whose_only_power_pin_has_no_stock_symbol_still_gets_a_power_flag() {
    // The MP2307's `in` is the rail's only power input and the part has no stock
    // symbol, so it cannot anchor the flag; the capacitor on the rail is passive
    // and has a stock symbol, and anchors it instead of the rail going unflagged.
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("registry");
    let filename = "buck_rail.synth";
    let src = r#"board "buck_rail" {
  layers 2
  power "VIN_RAIL" 12v
  component U1: regulator "mp2307_buck"
  component C1: capacitor "c_generic_0603" value "10uF"
  net "VIN_RAIL" { U1.in C1.p1 }
}
"#;
    let ast = synth_parser::parse(src, filename.to_string())
        .ast
        .expect("ast");
    let board = synth_ir::lower(&ast, &registry, filename)
        .board
        .expect("board");
    let layout = synth_layout::layout(&board);
    let placements: std::collections::HashMap<_, _> =
        layout.components.iter().map(|p| (p.id, p)).collect();
    let drivers = synth_kicad::pin_reconcile::undriven_power_nets(&board, &placements);
    let rail = drivers
        .iter()
        .find(|d| board.net(d.net).unwrap().name == "VIN_RAIL")
        .expect("VIN_RAIL is flagged though U1 cannot anchor it");
    assert_eq!(board.component(rail.anchor_component).unwrap().refdes, "C1");
}

#[test]
fn the_power_flag_of_a_rail_sits_on_the_end_of_a_wire() {
    // The flag joins the rail only where a wire ends on its pin; a flag anchored
    // on a turned part must be drawn at that part's turned pin, not its upright one.
    if synth_layout::kicad_lib_loader::physical_pins("Device:C").is_none() {
        eprintln!("KiCad symbol library not installed locally; skipping the flag position test");
        return;
    }
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("registry");
    let filename = "buck_rail.synth";
    let src = r#"board "buck_rail" {
  layers 2
  power "VIN_RAIL" 12v
  component U1: regulator "mp2307_buck"
  component C1: capacitor "c_generic_0603" value "10uF"
  component C2: capacitor "c_generic_0603" value "10uF"
  net "VIN_RAIL" { U1.in C1.p1 C2.p1 }
}
"#;
    let ast = synth_parser::parse(src, filename.to_string())
        .ast
        .expect("ast");
    let board = synth_ir::lower(&ast, &registry, filename)
        .board
        .expect("board");
    let dir = std::env::temp_dir().join(format!("synth-flag-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let out = synth_kicad::export(&board, &dir).expect("export");
    let sch = std::fs::read_to_string(out.pcb_path.with_extension("kicad_sch"))
        .expect(".kicad_sch written");
    let flag = sch
        .split("(lib_id \"power:PWR_FLAG\")")
        .nth(1)
        .and_then(|rest| rest.split("(at ").nth(1))
        .map(|rest| {
            rest.split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .expect("the rail has a PWR_FLAG");
    assert!(
        sch.contains(&format!("(xy {flag})")),
        "no wire ends at the flag's pin ({flag})"
    );
}
