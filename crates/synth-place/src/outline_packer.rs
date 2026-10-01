// SPDX-License-Identifier: Apache-2.0

//! Outline Island Network-Aware Passive Component Packer.
//!
//! Inspired by tscircuit's `calculate-packing` solver:
//! 1. Macro components (ICs, connectors, regulators) are placed first via CEM.
//! 2. Passives (resistors, capacitors, small diodes) are sorted largest → smallest.
//! 3. Maintains an outline hull (union of inflated AABBs of placed components).
//! 4. Probes boundary candidate positions that minimize distance to pads sharing `NetId`s.
//! 5. Evaluates 4 orthogonal rotations (0°, 90°, 180°, 270°) and selects the non-overlapping
//!    position with the lowest HPWL cost.

use std::collections::HashMap;

use synth_geometry::{mm_to_nm, Point, Rect, Rotation};
use synth_ir::{Board, ComponentId, NetId};

use crate::{hpwl_total, intersects_keepout, ComponentPlacement, PadOffsetLookup, PlaceError};

/// Clearance gap between component courtyards in nanometres (3.0 mm).
const CLEARANCE_GAP_NM: i64 = 3_000_000;

/// Grid pitch for boundary probing in nanometres (0.5 mm).
const PROBE_PITCH_NM: i64 = 500_000;

/// Outline Hull tracking placed component extents inflated by clearance gap.
#[derive(Debug, Clone)]
pub struct OutlineHull {
    pub placed_rects: Vec<Rect>,
}

impl Default for OutlineHull {
    fn default() -> Self {
        Self::new()
    }
}

impl OutlineHull {
    pub fn new() -> Self {
        Self {
            placed_rects: Vec::new(),
        }
    }

    /// Add a placed component to the outline hull.
    /// Inflate an already-computed courtyard rect by [`CLEARANCE_GAP_NM`] and
    /// record it as an obstacle.
    ///
    /// Takes the rect rather than a centre and half-extents because a placement
    /// anchor is footprint-relative: for a DIP or a long header the courtyard
    /// centre sits millimetres away from the anchor, so inflating an
    /// anchor-centred box builds a hull that does not actually cover the part.
    pub fn add_center(&mut self, courtyard: Rect) {
        self.placed_rects.push(Rect::new(
            Point::new(
                courtyard.min.x_nm - CLEARANCE_GAP_NM,
                courtyard.min.y_nm - CLEARANCE_GAP_NM,
            ),
            Point::new(
                courtyard.max.x_nm + CLEARANCE_GAP_NM,
                courtyard.max.y_nm + CLEARANCE_GAP_NM,
            ),
        ));
    }

    /// Generate candidate probe points along the exposed perimeter of placed components.
    pub fn sample_boundary_candidates(
        &self,
        target: Point,
        usable: Rect,
        half_w: i64,
        half_h: i64,
    ) -> Vec<Point> {
        let mut candidates = Vec::new();
        if self.placed_rects.is_empty() {
            candidates.push(target);
            return candidates;
        }

        // Sample points around the perimeter of each inflated rectangle in the hull
        for rect in &self.placed_rects {
            let min_x = rect.min.x_nm.max(usable.min.x_nm);
            let max_x = rect.max.x_nm.min(usable.max.x_nm);
            let min_y = rect.min.y_nm.max(usable.min.y_nm);
            let max_y = rect.max.y_nm.min(usable.max.y_nm);

            // Top edge (min_y - half_h) & bottom edge (max_y + half_h)
            let mut x = min_x;
            while x <= max_x {
                candidates.push(Point::new(x, min_y - half_h));
                candidates.push(Point::new(x, max_y + half_h));
                x += PROBE_PITCH_NM;
            }

            // Left edge (min_x - half_w) & right edge (max_x + half_w)
            let mut y = min_y;
            while y <= max_y {
                candidates.push(Point::new(min_x - half_w, y));
                candidates.push(Point::new(max_x + half_w, y));
                y += PROBE_PITCH_NM;
            }
        }

        // Sort candidates by Euclidean distance to target pad centroid
        candidates.sort_by_key(|p| (p.x_nm - target.x_nm).pow(2) + (p.y_nm - target.y_nm).pow(2));
        candidates.dedup();
        candidates.retain(|pt| {
            let cand_rect = Rect::from_center_half_extents(*pt, half_w, half_h);
            self.placed_rects.iter().all(|r| !cand_rect.intersects(r))
        });
        candidates.truncate(150); // Keep top 150 closest boundary candidates
        candidates
    }
}

/// Pack passive components along the outline hull of placed macros.
pub(crate) fn pack_passives_along_outline(
    board: &Board,
    placements: &mut Vec<ComponentPlacement>,
    passives: &[ComponentId],
    courtyard_lookup: &HashMap<ComponentId, (f64, f64)>,
    courtyard_offset_lookup: &HashMap<ComponentId, (f64, f64)>,
    pad_offsets: &PadOffsetLookup,
    usable: Rect,
) -> Result<(), PlaceError> {
    let mut hull = OutlineHull::new();

    // 1. Build initial hull from placed macro components
    for p in placements.iter() {
        let (w_mm, h_mm) = courtyard_lookup[&p.id];
        let offset = courtyard_offset_lookup.get(&p.id).copied();
        let rect = crate::courtyard_rect_for_placement(p, w_mm, h_mm, offset, 0);
        hull.add_center(rect);
    }

    // 2. Sort passives by area descending (largest passives first)
    let mut sorted_passives = passives.to_vec();
    sorted_passives.sort_by(|a, b| {
        let (wa, ha) = courtyard_lookup[a];
        let (wb, hb) = courtyard_lookup[b];
        (wb * hb).partial_cmp(&(wa * ha)).unwrap()
    });

    // 3. Map nets to placed pads for quick target centroid lookup
    let mut net_pad_positions: HashMap<NetId, Vec<Point>> = HashMap::new();
    update_net_pad_positions(board, placements, pad_offsets, &mut net_pad_positions);

    // 4. Pack each passive component
    for &comp_id in &sorted_passives {
        let (w_mm, h_mm) = courtyard_lookup[&comp_id];
        let unrot_half_w = mm_to_nm(w_mm) / 2;
        let unrot_half_h = mm_to_nm(h_mm) / 2;
        let passive_offset = courtyard_offset_lookup
            .get(&comp_id)
            .copied()
            .unwrap_or((0.0, 0.0));

        // Find connected target nets for this component
        let comp_nets = get_component_nets(board, comp_id);
        let target_point = compute_target_centroid(board, &comp_nets, &net_pad_positions)
            .unwrap_or_else(|| {
                Point::new(
                    (usable.min.x_nm + usable.max.x_nm) / 2,
                    (usable.min.y_nm + usable.max.y_nm) / 2,
                )
            });

        let candidates =
            hull.sample_boundary_candidates(target_point, usable, unrot_half_w, unrot_half_h);

        let mut best_choice: Option<(Point, Rotation, i64)> = None;

        for cand in &candidates {
            for &rot in &[
                Rotation::Zero,
                Rotation::Ninety,
                Rotation::OneEighty,
                Rotation::TwoSeventy,
            ] {
                let cand_rect = crate::courtyard_rect_at_anchor(
                    *cand,
                    rot,
                    unrot_half_w,
                    unrot_half_h,
                    passive_offset,
                    0,
                );

                // Check bounds, courtyard collisions, and keepout regions
                if !usable.contains(cand_rect.min) || !usable.contains(cand_rect.max) {
                    continue;
                }
                if intersects_placed(
                    cand_rect,
                    placements,
                    courtyard_lookup,
                    courtyard_offset_lookup,
                ) {
                    continue;
                }
                let temp_placed: Vec<(ComponentId, Rect)> = placements
                    .iter()
                    .map(|p| {
                        let (w, h) = courtyard_lookup[&p.id];
                        (
                            p.id,
                            crate::courtyard_rect_for_placement(
                                p,
                                w,
                                h,
                                courtyard_offset_lookup.get(&p.id).copied(),
                                0,
                            ),
                        )
                    })
                    .collect();
                if intersects_keepout(cand_rect, comp_id, board, usable, |pid| {
                    temp_placed
                        .iter()
                        .find(|(placed_id, _)| *placed_id == pid)
                        .map(|(_, r)| *r)
                }) {
                    continue;
                }

                // Temporary placement to score HPWL
                let trial = ComponentPlacement {
                    id: comp_id,
                    center: *cand,
                    rotation: rot,
                    layer: synth_geometry::Layer::Top,
                };
                placements.push(trial);
                let cost = hpwl_total(board, placements, pad_offsets);
                placements.pop();

                if best_choice
                    .as_ref()
                    .is_none_or(|(_, _, best_cost)| cost < *best_cost)
                {
                    best_choice = Some((*cand, rot, cost));
                }
            }
        }

        // Apply best placement or fallback legal position search
        if let Some((best_pt, best_rot, _)) = best_choice {
            let placement = ComponentPlacement {
                id: comp_id,
                center: best_pt,
                rotation: best_rot,
                layer: synth_geometry::Layer::Top,
            };
            placements.push(placement);
            hull.add_center(crate::courtyard_rect_for_placement(
                &placement,
                w_mm,
                h_mm,
                Some(passive_offset),
                0,
            ));
            update_net_pad_positions(board, placements, pad_offsets, &mut net_pad_positions);
        } else if let Some(fallback_pt) = find_nearest_legal_slot(
            comp_id,
            board,
            target_point,
            unrot_half_w,
            unrot_half_h,
            passive_offset,
            usable,
            placements,
            courtyard_lookup,
            courtyard_offset_lookup,
        ) {
            let fallback = ComponentPlacement {
                id: comp_id,
                center: fallback_pt,
                rotation: Rotation::Zero,
                layer: synth_geometry::Layer::Top,
            };
            placements.push(fallback);
            hull.add_center(crate::courtyard_rect_for_placement(
                &fallback,
                w_mm,
                h_mm,
                Some(passive_offset),
                0,
            ));
            update_net_pad_positions(board, placements, pad_offsets, &mut net_pad_positions);
        } else {
            let refdes = board
                .component(comp_id)
                .map_or_else(|| format!("#{}", comp_id.0), |c| c.refdes.clone());
            return Err(PlaceError::NoLegalPosition {
                refdes,
                board_w_mm: synth_geometry::nm_to_mm(usable.width_nm()),
                board_h_mm: synth_geometry::nm_to_mm(usable.height_nm()),
                tried: 150,
            });
        }
    }
    Ok(())
}

fn find_nearest_legal_slot(
    id: ComponentId,
    board: &Board,
    target: Point,
    unrot_half_w: i64,
    unrot_half_h: i64,
    passive_offset: (f64, f64),
    usable: Rect,
    placements: &[ComponentPlacement],
    courtyard_lookup: &HashMap<ComponentId, (f64, f64)>,
    courtyard_offset_lookup: &HashMap<ComponentId, (f64, f64)>,
) -> Option<Point> {
    let pitch = 500_000; // 0.5 mm grid step
    let temp_placed: Vec<(ComponentId, Rect)> = placements
        .iter()
        .map(|p| {
            let (w, h) = courtyard_lookup[&p.id];
            (
                p.id,
                crate::courtyard_rect_for_placement(
                    p,
                    w,
                    h,
                    courtyard_offset_lookup.get(&p.id).copied(),
                    0,
                ),
            )
        })
        .collect();

    for ring in 1..100 {
        let offset = ring * pitch;
        let coords = [
            Point::new(target.x_nm + offset, target.y_nm),
            Point::new(target.x_nm - offset, target.y_nm),
            Point::new(target.x_nm, target.y_nm + offset),
            Point::new(target.x_nm, target.y_nm - offset),
            Point::new(target.x_nm + offset, target.y_nm + offset),
            Point::new(target.x_nm - offset, target.y_nm - offset),
            Point::new(target.x_nm + offset, target.y_nm - offset),
            Point::new(target.x_nm - offset, target.y_nm + offset),
        ];
        for cand in coords {
            let cand_rect = crate::courtyard_rect_at_anchor(
                cand,
                Rotation::Zero,
                unrot_half_w,
                unrot_half_h,
                passive_offset,
                0,
            );
            if usable.contains(cand_rect.min)
                && usable.contains(cand_rect.max)
                && !intersects_placed(
                    cand_rect,
                    placements,
                    courtyard_lookup,
                    courtyard_offset_lookup,
                )
                && !intersects_keepout(cand_rect, id, board, usable, |pid| {
                    temp_placed
                        .iter()
                        .find(|(placed_id, _)| *placed_id == pid)
                        .map(|(_, r)| *r)
                })
            {
                return Some(cand);
            }
        }
    }
    None
}

fn get_component_nets(board: &Board, id: ComponentId) -> Vec<NetId> {
    let mut nets = Vec::new();
    for net in &board.nets {
        if net.endpoints.iter().any(|ep| ep.component == id) {
            nets.push(net.id);
        }
    }
    nets
}

fn compute_target_centroid(
    board: &Board,
    nets: &[NetId],
    net_pad_positions: &HashMap<NetId, Vec<Point>>,
) -> Option<Point> {
    let is_global_rail = |net_id: NetId| -> bool {
        let name = board
            .nets
            .iter()
            .find(|n| n.id == net_id)
            .map_or("", |n| n.name.as_str())
            .to_lowercase();
        name == "gnd"
            || name == "vcc"
            || name == "vdd"
            || name == "3v3"
            || name == "5v"
            || name == "vbus"
    };

    let local_nets: Vec<NetId> = nets
        .iter()
        .copied()
        .filter(|&id| !is_global_rail(id))
        .collect();
    let target_nets = if local_nets.is_empty() {
        nets
    } else {
        &local_nets
    };

    let mut sum_x = 0_i64;
    let mut sum_y = 0_i64;
    let mut count = 0_i64;

    for &net_id in target_nets {
        if let Some(pts) = net_pad_positions.get(&net_id) {
            for pt in pts {
                sum_x += pt.x_nm;
                sum_y += pt.y_nm;
                count += 1;
            }
        }
    }

    if count == 0 {
        None
    } else {
        Some(Point::new(sum_x / count, sum_y / count))
    }
}

fn update_net_pad_positions(
    board: &Board,
    placements: &[ComponentPlacement],
    pad_offsets: &PadOffsetLookup,
    net_pad_positions: &mut HashMap<NetId, Vec<Point>>,
) {
    net_pad_positions.clear();
    let place_map: HashMap<ComponentId, &ComponentPlacement> =
        placements.iter().map(|p| (p.id, p)).collect();

    for net in &board.nets {
        let mut pts = Vec::new();
        for ep in &net.endpoints {
            if let Some(p) = place_map.get(&ep.component) {
                let (off_x, off_y) = pad_offsets
                    .lookup(ep.component, ep.pin.0 as usize)
                    .unwrap_or((0, 0));
                let (rx, ry) = p.rotation.rotate_offset(off_x, off_y);
                pts.push(Point::new(p.center.x_nm + rx, p.center.y_nm + ry));
            }
        }
        if !pts.is_empty() {
            net_pad_positions.insert(net.id, pts);
        }
    }
}

fn intersects_placed(
    rect: Rect,
    placements: &[ComponentPlacement],
    courtyard_lookup: &HashMap<ComponentId, (f64, f64)>,
    courtyard_offset_lookup: &HashMap<ComponentId, (f64, f64)>,
) -> bool {
    let pad_buffer_nm: i64 = 3_000_000; // 3.0mm pad clearance buffer to prevent SMD pad THT pin overlaps
    for p in placements {
        let (w_mm, h_mm) = courtyard_lookup[&p.id];
        let p_rect = crate::courtyard_rect_for_placement(
            p,
            w_mm,
            h_mm,
            courtyard_offset_lookup.get(&p.id).copied(),
            pad_buffer_nm,
        );
        if rect.intersects(&p_rect) {
            return true;
        }
    }
    false
}
