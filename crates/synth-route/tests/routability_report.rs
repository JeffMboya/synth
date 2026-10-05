//! Routability report across the example designs.
//!
//! Prints placement HPWL next to the router's own cost (wall time, A* cells
//! expanded, unrouted nets, track length, vias) so a placement change can be
//! judged by how much easier it makes routing, not by eyeballing.
//!
//! Ignored by default because it is a diagnostic, not a gate. Run it in
//! release mode; a debug-build route takes minutes:
//!
//! `cargo test --release -p synth-route --test routability_report -- --ignored --nocapture`

use std::path::Path;
use std::time::Instant;

fn load_board(path: &str) -> synth_ir::Board {
    let source = std::fs::read_to_string(path).expect("read fixture");
    let file = path.to_string();
    let parse = synth_parser::parse(&source, file.clone());
    let ast = parse.ast.as_ref().expect("parse");
    let registry_dir = Path::new("../..").join("registry").join("parts");
    let registry = synth_registry::load_dir(&registry_dir).expect("registry");
    let loader = synth_ir::FsImportLoader {
        root: std::path::PathBuf::from("../.."),
    };
    let resolved = synth_ir::resolve_imports(ast, &loader, &file);
    let lowered = synth_ir::lower(&resolved.program, &registry, &file);
    lowered.board.expect("board")
}

#[test]
#[ignore = "diagnostic report, not a gate"]
fn routability_report() {
    let examples = [
        "../../examples/sensor_logger.synth",
        "../../examples/env_logger.synth",
        "../../fixtures/designs/secure_tracker.synth",
    ];
    for path in examples {
        if !Path::new(path).exists() {
            continue;
        }
        let board = load_board(path);
        let started = Instant::now();
        let placement = match synth_place::place(&board) {
            Ok(p) => p,
            Err(e) => {
                println!("--- {path}\n  PLACE FAILED {e:?}");
                continue;
            }
        };
        let place_s = started.elapsed().as_secs_f64();
        let score = synth_place::score::score_placement(&board, &placement);
        let started = Instant::now();
        let routing = synth_route::route(&board, &placement);
        let route_s = started.elapsed().as_secs_f64();
        let track_mm: f64 = routing
            .segments
            .iter()
            .map(|s| {
                let dx = synth_geometry::nm_to_mm(s.end.x_nm - s.start.x_nm);
                let dy = synth_geometry::nm_to_mm(s.end.y_nm - s.start.y_nm);
                dx.hypot(dy)
            })
            .sum();
        println!("--- {path}");
        println!(
            "  place {place_s:.2}s | route {route_s:.2}s | cells {} | unrouted {} | hpwl {:.1} mm | track {track_mm:.1} mm | vias {}",
            routing.cells_expanded,
            routing.unrouted_nets.len(),
            synth_geometry::nm_to_mm(score.hpwl_total_nm),
            routing.vias.len()
        );
        for u in &routing.unrouted_nets {
            println!("  UNROUTED: {}", u.net_name);
        }
    }
}
