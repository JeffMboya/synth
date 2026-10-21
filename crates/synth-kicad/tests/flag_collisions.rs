// SPDX-License-Identifier: Apache-2.0

//! Net label flags must not be drawn on top of each other or on top of another
//! pin's number.
//!
//! A flag hangs off a stub at its pin and runs away from it. Two pins whose
//! flags land in the same place (head-on pins of one net, or a pull-up's flag
//! landing in an IC's flag column) used to draw box over box, so the names
//! could not be read. The check goes through the real export and reads the
//! `(global_label)` positions back from the `.kicad_sch`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Flag box in mm: 1.12 per character plus 2.9, and 2.65 tall.
const FLAG_CHAR_MM: f64 = 1.12;
const FLAG_PAD_MM: f64 = 2.9;
const FLAG_H_MM: f64 = 2.65;
/// Boxes that overlap by less than this in either direction only touch:
/// flags on a 2.54 mm pin pitch are 0.11 mm taller than the pitch.
const TOUCH_MM: f64 = 0.2;
/// Half the room a pin number takes around the pin's terminal.
const PIN_NUMBER_HALF_MM: f64 = 1.27;

type Rect = (f64, f64, f64, f64);

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn overlap(a: Rect, b: Rect) -> bool {
    a.2.min(b.2) - a.0.max(b.0) > TOUCH_MM && a.3.min(b.3) - a.1.max(b.1) > TOUCH_MM
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

fn load_board(path: &Path) -> synth_ir::Board {
    let registry = synth_registry::load_dir(&workspace_root().join("registry").join("parts"))
        .expect("seed registry must load");
    let name = path.file_name().unwrap().to_string_lossy().to_string();
    let src = std::fs::read_to_string(path).unwrap();
    let parsed = synth_parser::parse(&src, name.clone());
    synth_ir::lower(&parsed.ast.expect("ast"), &registry, &name)
        .board
        .expect("board")
}

/// What is wrong with the flags `board` exports, one line per collision.
fn collisions(board: &synth_ir::Board) -> Vec<String> {
    let dir = tempfile::tempdir().unwrap();
    let result = synth_kicad::export_schematic_only(board, dir.path(), None).expect("export");
    let sch = std::fs::read_to_string(&result.schematic_path).unwrap();
    let flags = flags(&sch);
    let mut found = Vec::new();
    for (i, (a_name, a)) in flags.iter().enumerate() {
        for (b_name, b) in &flags[i + 1..] {
            if overlap(*a, *b) {
                found.push(format!("{a_name} {a:?} on {b_name} {b:?}"));
            }
        }
    }
    // Pin positions are only the sheet's own on a single-sheet export.
    if result.sheet_count == 1 {
        let layout = synth_layout::layout_with_sidecar(board, None);
        let placements: std::collections::HashMap<_, _> =
            layout.components.iter().map(|p| (p.id, p)).collect();
        for component in &board.components {
            let Some(part) = component.part.as_ref() else {
                continue;
            };
            for index in 0..part.pins.len() {
                let pin = synth_ir::PinId(index as u32);
                let Some((x, y, ..)) =
                    synth_layout::route::pin_terminal_xy(board, component.id, pin, &placements)
                else {
                    continue;
                };
                let number = (
                    x - PIN_NUMBER_HALF_MM,
                    y - PIN_NUMBER_HALF_MM,
                    x + PIN_NUMBER_HALF_MM,
                    y + PIN_NUMBER_HALF_MM,
                );
                for (name, flag) in &flags {
                    if overlap(number, *flag) {
                        found.push(format!(
                            "{name} {flag:?} covers {}.{index} at ({x}, {y})",
                            component.refdes
                        ));
                    }
                }
            }
        }
    }
    found
}

fn reference_boards() -> Vec<PathBuf> {
    let dir = workspace_root().join("fixtures").join("kicad-reference");
    let mut sources: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "synth"))
        .collect();
    sources.sort();
    sources
}

/// The pins each net joins in KiCad's netlist of `board`'s schematic, as
/// `refdes.number` sets of two or more pins. `None` when kicad-cli is missing.
fn kicad_nets(board: &synth_ir::Board) -> Option<BTreeSet<BTreeSet<String>>> {
    let dir = tempfile::tempdir().unwrap();
    let result = synth_kicad::export_schematic_only(board, dir.path(), None).expect("export");
    let netlist = dir.path().join("probe.net");
    let run = Command::new("kicad-cli")
        .args([
            "sch",
            "export",
            "netlist",
            "--format",
            "kicadsexpr",
            "--output",
        ])
        .arg(&netlist)
        .arg(&result.schematic_path)
        .output()
        .ok()?;
    assert!(run.status.success(), "kicad-cli failed");
    let text = std::fs::read_to_string(&netlist).unwrap();
    let nets = text.split("(nets").nth(1).expect("nets");
    let quoted = |chunk: &str, key: &str| {
        chunk
            .split(key)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .map(str::to_string)
    };
    let joined = nets
        .split("(code")
        .skip(1)
        .map(|net| {
            net.split("(node")
                .skip(1)
                .map(|node| {
                    format!(
                        "{}.{}",
                        quoted(node, "(ref \"").unwrap(),
                        quoted(node, "(pin \"").unwrap()
                    )
                })
                .collect::<BTreeSet<_>>()
        })
        .filter(|pins| pins.len() > 1)
        .collect();
    Some(joined)
}

/// The pins each net of `board` joins, as `refdes.number` sets of two or more.
fn board_nets(board: &synth_ir::Board) -> Vec<BTreeSet<String>> {
    board
        .nets
        .iter()
        .map(|net| {
            net.endpoints
                .iter()
                .map(|ep| {
                    let component = board.component(ep.component).unwrap();
                    let part = component.part.as_ref().unwrap();
                    format!(
                        "{}.{}",
                        component.refdes, part.pins[ep.pin.0 as usize].number.0
                    )
                })
                .collect::<BTreeSet<_>>()
        })
        .filter(|pins| pins.len() > 1)
        .collect()
}

/// A flag moved to a new spot reaches over other pins' wires. One that touches
/// a wire joins two nets, and one that stops short of its pin splits one. The
/// boards are ones with a flag off its default spot, by reach, rise and side.
#[test]
fn moving_flags_neither_joins_two_nets_nor_splits_one() {
    for name in [
        "ref_ldo_sensor",
        "ref_crystal_mcu",
        "secure_tracker",
        "ref_usb_cdc",
    ] {
        let path = workspace_root()
            .join("fixtures")
            .join("kicad-reference")
            .join(format!("{name}.synth"));
        let board = load_board(&path);
        let Some(kicad) = kicad_nets(&board) else {
            eprintln!("kicad-cli not installed; skipping netlist test");
            return;
        };
        let nets = board_nets(&board);
        for joined in &kicad {
            let owners = nets.iter().filter(|net| !net.is_disjoint(joined)).count();
            assert!(
                owners <= 1,
                "{name}: KiCad joins {joined:?}, of {owners} nets"
            );
        }
        for net in &nets {
            let parts = kicad
                .iter()
                .filter(|joined| !joined.is_disjoint(net))
                .count();
            assert_eq!(parts, 1, "{name}: net {net:?} is in {parts} KiCad nets");
        }
    }
}

#[test]
fn no_flag_is_drawn_on_another_flag_or_on_a_pin_number_in_the_reference_boards() {
    let mut report = Vec::new();
    for path in reference_boards() {
        let file = path.file_name().unwrap().to_string_lossy().to_string();
        let found = collisions(&load_board(&path));
        report.extend(found.into_iter().map(|line| format!("{file}: {line}")));
    }
    assert!(report.is_empty(), "colliding flags:\n{}", report.join("\n"));
}
