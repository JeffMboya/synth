// SPDX-License-Identifier: Apache-2.0

//! A pull resistor stands beside the pin it serves, and net label flags hang
//! off the neighbouring pins on that same side. The resistor's body and its
//! wire must stay out of those flags, or the flags cannot be read and the wire
//! looks joined to them. The check goes through the real export and reads the
//! `(global_label)` positions back from the `.kicad_sch`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Flag box in mm: 1.12 per character plus 2.9, and 2.65 tall.
const FLAG_CHAR_MM: f64 = 1.12;
const FLAG_PAD_MM: f64 = 2.9;
const FLAG_H_MM: f64 = 2.65;
/// Half the width of a resistor body across its length, in mm.
const BODY_HALF_W_MM: f64 = 1.0;
/// Boxes that overlap by less than this in either direction only touch.
const TOUCH_MM: f64 = 0.2;

type Rect = (f64, f64, f64, f64);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

/// A modem-style IC whose pins carry flagged nets, with a pull-down on one pin
/// among them.
const PULL_DOWN: &str = r#"board "pull_flags" {
  layers 2
  manufacturer "jlcpcb"

  component U1: ic "ch340g"
  component J2: connector "header_1x4"
  component R9: resistor "r_generic_0603"

  net "GND" {
    U1.gnd, R9.p2
  }

  net "MODEM_IDLE" {
    U1.ri, U1.dsr, U1.dcd, J2.p1
  }

  connect U1.cts -> R9.p1 as "CTS_LOW"
  connect U1.dtr -> J2.p2 as "DTR_OUT"
}
"#;

/// The same IC with a pull-up to a rail on the pins of one flagged net.
const PULL_UP: &str = r#"board "pull_flags" {
  layers 2
  manufacturer "jlcpcb"

  component U1: ic "ch340g"
  component J2: connector "header_1x4"
  component R8: resistor "r_generic_0603"

  net "VCC" {
    U1.vcc, R8.p1
  }

  net "MODEM_IDLE" {
    U1.ri, U1.dsr, U1.dcd, R8.p2
  }

  connect U1.cts -> J2.p1 as "CTS_OUT"
  connect U1.dtr -> J2.p2 as "DTR_OUT"
}
"#;

fn load_board(source: &str) -> synth_ir::Board {
    let registry =
        synth_registry::load_dir(&workspace_root().join("registry").join("parts")).unwrap();
    let ast = synth_parser::parse(source, "pull_flags.synth".to_string())
        .ast
        .unwrap();
    synth_ir::lower(&ast, &registry, "pull_flags.synth")
        .board
        .unwrap()
}

/// Every global label in `sch` as (name, box).
#[allow(clippy::cast_precision_loss)] // a net name is a handful of characters
fn flags(sch: &str) -> Vec<(String, Rect)> {
    let mut out = Vec::new();
    for block in sch.split("(global_label").skip(1) {
        let name = block.split('"').nth(1).expect("label name");
        let at = block.split("(at ").nth(1).expect("label position");
        let mut nums = at
            .split(|c: char| c.is_whitespace() || c == ')')
            .filter_map(|t| t.parse::<f64>().ok());
        let (x, y, angle) = (
            nums.next().unwrap(),
            nums.next().unwrap(),
            nums.next().unwrap(),
        );
        let w = name.chars().count() as f64 * FLAG_CHAR_MM + FLAG_PAD_MM;
        let (x0, x1) = if angle == 180.0 {
            (x - w, x)
        } else {
            (x, x + w)
        };
        out.push((
            name.to_string(),
            (x0, y - FLAG_H_MM / 2.0, x1, y + FLAG_H_MM / 2.0),
        ));
    }
    out
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.2.min(b.2) - a.0.max(b.0) > TOUCH_MM && a.3.min(b.3) - a.1.max(b.1) > TOUCH_MM
}

/// What a flag covers of `resistor`'s body or of any wire, in the export of `source`.
fn flags_in_the_way(source: &str, resistor: &str) -> Vec<String> {
    let board = load_board(source);
    let dir = tempfile::tempdir().unwrap();
    let result = synth_kicad::export_schematic_only(&board, dir.path(), None).expect("export");
    let sch = std::fs::read_to_string(&result.schematic_path).unwrap();
    let flags = flags(&sch);
    assert!(flags.len() >= 4, "the board is meant to carry flags");

    let layout = synth_layout::layout_with_sidecar(&board, None);
    let placements: HashMap<_, _> = layout.components.iter().map(|p| (p.id, p)).collect();
    let mut found = Vec::new();
    let part = board
        .components
        .iter()
        .find(|c| c.refdes == resistor)
        .unwrap();
    let ends: Vec<(f64, f64)> = (0..2)
        .map(|pin| {
            let (x, y, ..) = synth_layout::route::pin_terminal_xy(
                &board,
                part.id,
                synth_ir::PinId(pin),
                &placements,
            )
            .unwrap();
            (x, y)
        })
        .collect();
    // The body lies between its two terminals and is a little wider than the
    // line they span.
    let (mut x0, mut y0) = (ends[0].0.min(ends[1].0), ends[0].1.min(ends[1].1));
    let (mut x1, mut y1) = (ends[0].0.max(ends[1].0), ends[0].1.max(ends[1].1));
    if x1 - x0 < y1 - y0 {
        (x0, x1) = (x0 - BODY_HALF_W_MM, x1 + BODY_HALF_W_MM);
    } else {
        (y0, y1) = (y0 - BODY_HALF_W_MM, y1 + BODY_HALF_W_MM);
    }
    let body = (x0, y0, x1, y1);
    for (name, flag) in &flags {
        if overlap(body, *flag) {
            found.push(format!("{name} {flag:?} covers {resistor} body {body:?}"));
        }
    }
    for wire in &layout.wires {
        let net = &board.nets[wire.net.0 as usize].name;
        for segment in wire.points.windows(2) {
            let rect = (
                segment[0].0.min(segment[1].0),
                segment[0].1.min(segment[1].1),
                segment[0].0.max(segment[1].0),
                segment[0].1.max(segment[1].1),
            );
            for (name, flag) in &flags {
                // A segment has no width: it crosses a flag when it passes
                // through the box, not when it only runs beside it.
                let through = rect.2 > flag.0 + TOUCH_MM
                    && rect.0 < flag.2 - TOUCH_MM
                    && rect.3 > flag.1 + TOUCH_MM
                    && rect.1 < flag.3 - TOUCH_MM;
                if through && name != net {
                    found.push(format!(
                        "{name} {flag:?} crossed by a wire of {net} {rect:?}"
                    ));
                }
            }
        }
    }
    found
}

#[test]
fn no_flag_is_drawn_over_a_pull_down_or_its_wire() {
    let found = flags_in_the_way(PULL_DOWN, "R9");
    assert!(found.is_empty(), "flags in the way:\n{}", found.join("\n"));
}

#[test]
fn no_flag_is_drawn_over_a_pull_up_or_its_wire() {
    let found = flags_in_the_way(PULL_UP, "R8");
    assert!(found.is_empty(), "flags in the way:\n{}", found.join("\n"));
}
