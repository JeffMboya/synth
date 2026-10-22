// SPDX-License-Identifier: Apache-2.0

//! Two things the exporter has to get right about a footprint it did not
//! draw: where that footprint's pads are, and where its rule areas are.
//!
//! Both used to be wrong, and both were wrong silently — the board exported,
//! the schematic matched, and only `kicad-cli pcb drc` on the placed copper
//! said so, 100 mm away from the part at fault.

use std::fs;
use std::path::{Path, PathBuf};

/// A MAX3485 is a SOIC-8 whose `V3V3` supply is pin 8, a corner pad. The
/// synthesized fallback geometry would put that pin in the middle of a
/// two-column strip instead, over 3 mm away.
const PLANE_SOURCE: &str = r#"board "plane_stub" {
  layers 4
  manufacturer "jlc-advanced"
  requires via_in_pad

  netclass "V3V3" {
    trace_width 0.6mm
    clearance 0.15mm
    plane
  }

  component U1: ic "max3485_rs485"
  component U2: ic "max3485_rs485"
  component C1: capacitor "c_generic_0603" value "100nF"
  component C2: capacitor "c_generic_0603" value "100nF"
  component C3: capacitor "c_generic_0603" value "100nF"
  component C4: capacitor "c_generic_0603" value "100nF"
  component R1: resistor "r_generic_0603" value "10k"
  component R2: resistor "r_generic_0603" value "10k"

  connect U1.vcc -> C1.p1 as "V3V3" class "V3V3"
  connect C1.p2 -> U1.gnd as "GND"
  connect U2.vcc -> C2.p1
  connect C2.p2 -> U2.gnd as "GND"
  connect C3.p1 -> C1.p1
  connect C3.p2 -> C1.p2
  connect C4.p1 -> C2.p1
  connect C4.p2 -> C2.p2
  connect U1.ro -> R1.p1
  connect R1.p2 -> U1.di
  connect U2.ro -> R2.p1
  connect R2.p2 -> U2.di
}
"#;

/// A microSD socket carries five keepout zones in its library module, around
/// the card's insertion envelope. The board does not place it at the origin,
/// so the exported zones must not be there either.
const SOCKET_SOURCE: &str = r#"board "socket_keepout" {
  layers 4
  component J1: connector "microsd_dm3at"
  component C1: capacitor "c_generic_0603" value "100nF"
  connect J1.vdd -> C1.p1
  connect C1.p2 -> J1.vss as "GND"
}
"#;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root")
}

/// Whether the stock KiCad footprint libraries are reachable. Without them
/// every footprint falls back to synthesized geometry and neither test has a
/// subject, so they report success rather than a false failure — the same
/// guard the other library-dependent tests use.
fn libraries_available() -> bool {
    synth_layout::kicad_footprint_loader::pads("Package_SO:SOIC-8_3.9x4.9mm_P1.27mm").is_some()
        && synth_layout::kicad_footprint_loader::pads(
            "Connector_Card:microSD_HC_Hirose_DM3AT-SF-PEJM5",
        )
        .is_some()
}

fn export_pcb(source: &str, label: &str) -> String {
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("seed registry must load");
    let file = format!("{label}.synth");
    let parsed = synth_parser::parse(source, file.clone());
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lowered = synth_ir::lower(&parsed.ast.expect("ast"), &registry, &file);
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let board = lowered.board.expect("board");

    let out = std::env::temp_dir().join(format!(
        "synth-kicad-fp-geom-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&out);
    let result = synth_kicad::export(&board, &out).expect("export");
    let text = fs::read_to_string(&result.pcb_path).expect(".kicad_pcb written");
    let _ = fs::remove_dir_all(&result.out_dir);
    text
}

/// The `(zone ...)` blocks of `text` — the rule areas, not the
/// `(zone_connect N)` a pad carries, and matched without assuming what
/// whitespace KiCad puts after the head.
fn zone_blocks(text: &str) -> Vec<&str> {
    blocks(text, "(zone")
        .into_iter()
        .filter(|z| z["(zone".len()..].starts_with(|c: char| c.is_whitespace() || c == '('))
        .collect()
}

/// One balanced s-expression starting at each `prefix` in `text`.
fn blocks<'a>(text: &'a str, prefix: &str) -> Vec<&'a str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(idx) = text[from..].find(prefix) {
        let start = from + idx;
        let mut depth = 0_i32;
        let mut end = None;
        for (i, b) in bytes.iter().enumerate().skip(start) {
            match b {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        match end {
            Some(e) => {
                out.push(&text[start..e]);
                from = e;
            }
            None => break,
        }
    }
    out
}

/// The numbers in the first `(<key> x y ...)` of `block`.
fn xy_after(block: &str, key: &str) -> Option<(f64, f64)> {
    let idx = block.find(key)?;
    let mut tokens = block[idx + key.len()..].split_whitespace();
    let x = tokens.next()?.trim_end_matches(')').parse().ok()?;
    let y = tokens.next()?.trim_end_matches(')').parse().ok()?;
    Some((x, y))
}

/// Every `(xy x y)` in `block`.
fn xy_points(block: &str) -> Vec<(f64, f64)> {
    blocks(block, "(xy ")
        .iter()
        .filter_map(|p| xy_after(p, "(xy "))
        .collect()
}

/// Board-space pad centres of every footprint in an exported board, as
/// `(refdes, pad number, x, y)`. A quarter turn is KiCad's file convention,
/// the one `synth_geometry::Rotation::rotate_offset` encodes.
fn pad_centres(pcb: &str) -> Vec<(String, String, f64, f64)> {
    let mut out = Vec::new();
    for fp in blocks(pcb, "(footprint") {
        let Some(refdes) = quoted_after(fp, "\"Reference\"") else {
            continue;
        };
        let Some((ox, oy)) = xy_after(fp, "(at ") else {
            continue;
        };
        let angle = blocks(fp, "(at ")
            .first()
            .and_then(|at| at.split_whitespace().nth(3))
            .and_then(|a| a.trim_end_matches(')').parse::<f64>().ok())
            .unwrap_or(0.0);
        let radians = -angle.to_radians();
        for pad in blocks(fp, "(pad ") {
            let Some(number) = quoted_after(pad, "(pad") else {
                continue;
            };
            let Some((px, py)) = xy_after(pad, "(at ") else {
                continue;
            };
            out.push((
                refdes.clone(),
                number,
                ox + px * radians.cos() - py * radians.sin(),
                oy + px * radians.sin() + py * radians.cos(),
            ));
        }
    }
    out
}

/// The first quoted string after `key`.
fn quoted_after(text: &str, key: &str) -> Option<String> {
    let idx = text.find(key)? + key.len();
    let open = text[idx..].find('"')? + idx + 1;
    let close = text[open..].find('"')? + open;
    Some(text[open..close].to_string())
}

#[test]
fn a_plane_stub_starts_on_the_pad_the_exporter_emitted() {
    if !libraries_available() {
        return;
    }
    let pcb = export_pcb(PLANE_SOURCE, "plane-stub");
    let centres = pad_centres(&pcb);
    assert!(
        centres.iter().any(|(r, n, ..)| r == "U1" && n == "8"),
        "the SOIC-8's pin 8 must be on the board"
    );

    // A fanout stub is the only PLANE_STUB_WIDTH_NM track a routerless
    // export writes, so every 0.25 mm segment is one.
    let stubs: Vec<(f64, f64)> = blocks(&pcb, "(segment")
        .iter()
        .filter(|s| s.contains("(width 0.25)"))
        .filter_map(|s| xy_after(s, "(start "))
        .collect();
    assert!(
        !stubs.is_empty(),
        "a declared plane must fan its pads out to the plane"
    );

    for (sx, sy) in &stubs {
        let nearest = centres
            .iter()
            .map(|(r, n, x, y)| ((x - sx).hypot(y - sy), r, n))
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .expect("the board has pads");
        assert!(
            nearest.0 < 0.001,
            "a stub at ({sx:.3}, {sy:.3}) starts {:.3} mm off its nearest pad \
             ({} pad {}): the exporter sited it against geometry it did not emit",
            nearest.0,
            nearest.1,
            nearest.2,
        );
    }
}

#[test]
fn a_footprints_keepout_moves_with_the_footprint() {
    if !libraries_available() {
        return;
    }
    let pcb = export_pcb(SOCKET_SOURCE, "socket-keepout");

    let socket = blocks(&pcb, "(footprint")
        .into_iter()
        .find(|fp| quoted_after(fp, "\"Reference\"").as_deref() == Some("J1"))
        .expect("the socket is on the board");
    let (ox, oy) = xy_after(socket, "(at ").expect("the socket is placed");
    assert!(
        ox.hypot(oy) > 5.0,
        "the test is only meaningful with the socket away from the origin"
    );

    let zones: Vec<&str> = zone_blocks(socket)
        .into_iter()
        .filter(|z| z.contains("keepout"))
        .collect();
    assert!(
        !zones.is_empty(),
        "the DM3AT module carries keepout zones; without them there is \
         nothing to transform"
    );

    // The module's own pads bound where the part is. A rule area the part
    // carries belongs over the part, so it has to land inside that span —
    // untransformed library coordinates would put it over the board origin.
    let pads: Vec<(f64, f64)> = pad_centres(&pcb)
        .into_iter()
        .filter(|(r, ..)| r == "J1")
        .map(|(_, _, x, y)| (x, y))
        .collect();
    let (min_x, max_x, min_y, max_y) = pads.iter().fold(
        (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
        |(a, b, c, d), (x, y)| (a.min(*x), b.max(*x), c.min(*y), d.max(*y)),
    );

    for zone in zones {
        for (x, y) in xy_points(zone) {
            assert!(
                x > min_x - 10.0 && x < max_x + 10.0 && y > min_y - 10.0 && y < max_y + 10.0,
                "a keepout point ({x:.2}, {y:.2}) is nowhere near the socket \
                 it belongs to (pads span {min_x:.2}..{max_x:.2}, \
                 {min_y:.2}..{max_y:.2}): it is still in footprint-local \
                 coordinates"
            );
        }
    }
}
