//! Placement quality report across the example designs.
//!
//! Prints objective metrics so a placement change can be compared rather than
//! eyeballed: HPWL, decoupling distance, connector edge distance, board area,
//! and the structural visual-review findings.
//!
//! Ignored by default because it is a diagnostic, not a gate.

use std::path::Path;

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
    let lowered = synth_ir::lower(&resolved.program, &registry, &file);
    lowered.board.expect("board")
}

#[test]
#[ignore = "diagnostic report, not a gate"]
fn placement_quality_report() {
    let examples = [
        "../../examples/sensor_logger.synth",
        "../../examples/env_logger.synth",
        "../../examples/placement_and_diff_pair.synth",
        "../../fixtures/designs/secure_tracker.synth",
    ];
    for path in examples {
        if !Path::new(path).exists() {
            continue;
        }
        let board = load_board(path);
        let placement = match synth_place::place(&board) {
            Ok(p) => p,
            Err(e) => {
                println!("--- {path}\n  PLACE FAILED {e:?}");
                continue;
            }
        };
        let desc = synth_place::describe_placement(&board, &placement);
        let score = synth_place::score::score_placement(&board, &placement);
        println!("--- {path}");
        println!(
            "  outline {:.1}x{:.1} mm | components {} | visual_score {} | requires_revision {}",
            desc.board_size_mm[0],
            desc.board_size_mm[1],
            placement.components.len(),
            desc.visual_review.score,
            desc.visual_review.requires_revision
        );
        println!(
            "  hpwl {:.1} mm | max_decap {:.2} mm | avg_decap {:.2} mm | top_rail_ratio {:.2} | min_connector_edge {:.2} mm",
            synth_geometry::nm_to_mm(score.hpwl_total_nm),
            score.max_decoupling_distance_mm,
            score.avg_decoupling_distance_mm,
            score.top_rail_passive_ratio,
            score.edge_connector_boundary_dist_mm
        );
        println!(
            "  passes_human_quality_gate={}",
            score.passes_human_quality_gate
        );
        for f in &desc.visual_review.findings {
            println!("  FINDING: {f}");
        }
        for w in &desc.functional_warnings {
            println!("  FUNC: {w}");
        }
        for d in &desc.dense_regions {
            println!("  DENSE: {} {:.2}", d.region, d.density);
        }
    }
}
