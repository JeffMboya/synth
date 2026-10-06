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

/// How many legal slots a pad-bound passive may be chosen from.
///
/// Small enough that the closest slots — the ones on the side of the anchor
/// its rail pin faces — decide the placement, large enough that a blocked
/// side still finds room nearby instead of falling back to the far side of
/// the board.
const BOUND_CANDIDATE_BUDGET: usize = 8;

/// Outline Hull tracking placed component extents inflated by clearance gap.
///
/// The owning component id travels with each rect so a caller can tell which
/// body a slot is being measured against.
#[derive(Debug, Clone)]
pub struct OutlineHull {
    pub placed_rects: Vec<(ComponentId, Rect)>,
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
        self.add_center_for(ComponentId(u32::MAX), courtyard);
    }

    /// [`Self::add_center`] with the component the rect belongs to.
    pub fn add_center_for(&mut self, component: ComponentId, courtyard: Rect) {
        self.placed_rects.push((
            component,
            Rect::new(
                Point::new(
                    courtyard.min.x_nm - CLEARANCE_GAP_NM,
                    courtyard.min.y_nm - CLEARANCE_GAP_NM,
                ),
                Point::new(
                    courtyard.max.x_nm + CLEARANCE_GAP_NM,
                    courtyard.max.y_nm + CLEARANCE_GAP_NM,
                ),
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
        self.sample_boundary_candidates_except(target, usable, half_w, half_h, None)
    }

    /// [`Self::sample_boundary_candidates`] ignoring one placed body.
    ///
    /// The hull keeps a [`CLEARANCE_GAP_NM`] routing channel around every
    /// body, which is a real constraint — a trace has to run between parts.
    /// But a member bound to an anchor's pad belongs *against* that anchor,
    /// and the channel reserved around the anchor is exactly where it
    /// belongs. Exempting the anchor lets its members be sampled against its
    /// own perimeter, still clearing every other body by the full gap.
    pub fn sample_boundary_candidates_except(
        &self,
        target: Point,
        usable: Rect,
        half_w: i64,
        half_h: i64,
        exempt: Option<ComponentId>,
    ) -> Vec<Point> {
        let mut candidates = Vec::new();
        if self.placed_rects.is_empty() {
            candidates.push(target);
            return candidates;
        }

        // Sample points around the perimeter of each inflated rectangle in the hull
        for rect in self.placed_rects.iter().map(|(_, r)| r) {
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
            self.placed_rects
                .iter()
                .filter(|(id, _)| Some(*id) != exempt)
                .all(|(_, r)| !cand_rect.intersects(r))
        });
        candidates.truncate(150); // Keep top 150 closest boundary candidates
        candidates
    }
}

/// How far a bound part may land from its pad aim before the aim is
/// treated as unhonourable and the net-centroid search takes over.
///
/// A cap beside its pad and a cap 15 mm away are not the same placement, so
/// the aim is only worth following when it lands genuinely close; past this
/// the centroid — which knows nothing about the binding but at least sits
/// in open space — is the better of the two.
const BOUND_MAX_AIM_DISTANCE_NM: i64 = 8_000_000;

/// Whether a candidate slot at `cand` with `rotation` is legal: inside the
/// usable outline, clear of every placed courtyard, and clear of keepouts.
///
/// One predicate for both selection paths so "legal" cannot mean two
/// different things in the aim search and the fallback search.
#[allow(clippy::too_many_arguments)]
fn slot_is_legal(
    cand: Point,
    rotation: Rotation,
    comp_id: ComponentId,
    board: &Board,
    unrot_half_w: i64,
    unrot_half_h: i64,
    passive_offset: (f64, f64),
    usable: Rect,
    placements: &[ComponentPlacement],
    courtyard_lookup: &HashMap<ComponentId, (f64, f64)>,
    courtyard_offset_lookup: &HashMap<ComponentId, (f64, f64)>,
) -> bool {
    let cand_rect = crate::courtyard_rect_at_anchor(
        cand,
        rotation,
        unrot_half_w,
        unrot_half_h,
        passive_offset,
        0,
    );
    if !usable.contains(cand_rect.min) || !usable.contains(cand_rect.max) {
        return false;
    }
    if intersects_placed(
        cand_rect,
        placements,
        courtyard_lookup,
        courtyard_offset_lookup,
    ) {
        return false;
    }
    !intersects_keepout(cand_rect, comp_id, board, usable, |pid| {
        placements.iter().find(|p| p.id == pid).map(|p| {
            let (w, h) = courtyard_lookup[&p.id];
            crate::courtyard_rect_for_placement(
                p,
                w,
                h,
                courtyard_offset_lookup.get(&p.id).copied(),
                0,
            )
        })
    })
}

/// Where a passive bound to an anchor pad should sit, and which anchor it
/// may be placed against.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PadAim {
    pub anchor: ComponentId,
    pub placement: crate::modules::PadBoundPlacement,
}

/// Pack passive components along the outline hull of placed macros.
///
/// `pad_aims` carries, for each passive bound to an anchor pad, where it
/// should sit, how it should be turned, and which anchor it belongs to. A
/// bound passive aims at its pad instead of at the centroid of the nets it
/// touches, so a decoupling capacitor lands against the rail pad it
/// decouples rather than wherever the shared rail's centroid happens to be
/// — which, on a rail feeding five parts, is nowhere near any of them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn pack_passives_along_outline(
    board: &Board,
    placements: &mut Vec<ComponentPlacement>,
    passives: &[ComponentId],
    courtyard_lookup: &HashMap<ComponentId, (f64, f64)>,
    courtyard_offset_lookup: &HashMap<ComponentId, (f64, f64)>,
    pad_offsets: &PadOffsetLookup,
    child_module_map: &HashMap<ComponentId, crate::ChildPlacement>,
    usable: Rect,
) -> Result<(), PlaceError> {
    let mut hull = OutlineHull::new();

    // 1. Build initial hull from placed macro components
    for p in placements.iter() {
        let (w_mm, h_mm) = courtyard_lookup[&p.id];
        let offset = courtyard_offset_lookup.get(&p.id).copied();
        let rect = crate::courtyard_rect_for_placement(p, w_mm, h_mm, offset, 0);
        hull.add_center_for(p.id, rect);
    }

    // 2. Sort passives: bound members first, then the rest by area descending.
    //
    // A bound member is placed at the slot its anchor's pad names, so it has to
    // claim that slot before something else does. Packing by area alone gives no
    // such guarantee — a crystal's load capacitors are small, so they were
    // placed after every unrelated part had taken the space around the crystal,
    // and ended up wherever was left.
    let is_bound_member = |id: ComponentId| child_module_map.contains_key(&id);
    let mut sorted_passives = passives.to_vec();
    sorted_passives.sort_by(|a, b| {
        let bound_a = is_bound_member(*a);
        let bound_b = is_bound_member(*b);
        if bound_a != bound_b {
            return bound_b.cmp(&bound_a);
        }
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
        // The pad aim is resolved against the *live* placements rather than
        // a map built before packing started: a passive can be the anchor of
        // another passive — a crystal and its load capacitors — and such an
        // anchor is not placed yet when the aim map is built.
        let pad_aim = child_module_map.get(&comp_id).and_then(|child| {
            let placed: Vec<(ComponentId, Point, Rect)> = placements
                .iter()
                .map(|p| {
                    let (w, h) = courtyard_lookup[&p.id];
                    (
                        p.id,
                        p.center,
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
            let rotations: HashMap<ComponentId, Rotation> =
                placements.iter().map(|p| (p.id, p.rotation)).collect();
            crate::child_pad_bound_placement(
                child,
                &placed,
                &rotations,
                pad_offsets,
                courtyard_lookup,
                board,
            )
            .map(|placement| PadAim {
                anchor: child.anchor,
                placement,
            })
        });
        let target_point = match pad_aim {
            Some(aim) => aim.placement.center,
            None => compute_target_centroid(board, &comp_nets, &net_pad_positions).unwrap_or_else(
                || {
                    Point::new(
                        (usable.min.x_nm + usable.max.x_nm) / 2,
                        (usable.min.y_nm + usable.max.y_nm) / 2,
                    )
                },
            ),
        };

        // A bound passive first tries the slot nearest its own anchor pad. The
        // pad is the right target — a rail net reaches every part on the
        // board, so its centroid is nowhere near the part that needs the
        // capacitor — but only if a slot that close actually exists: where
        // the pad faces a congested region, chasing it can push the part
        // further from everything than the centroid would have. So the aim
        // is a preference with a distance bound, and the centroid search
        // below still runs when it cannot be honoured.
        let aim_choice = pad_aim.and_then(|aim| {
            let aim_point = aim.placement.center;
            let slots = hull.sample_boundary_candidates_except(
                aim_point,
                usable,
                unrot_half_w,
                unrot_half_h,
                Some(aim.anchor),
            );
            let rotations = [
                aim.placement.rotation,
                Rotation::Zero,
                Rotation::Ninety,
                Rotation::OneEighty,
                Rotation::TwoSeventy,
            ];
            slots
                .iter()
                .take(BOUND_CANDIDATE_BUDGET)
                .find(|cand| {
                    rotations.iter().any(|rot| {
                        slot_is_legal(
                            **cand,
                            *rot,
                            comp_id,
                            board,
                            unrot_half_w,
                            unrot_half_h,
                            passive_offset,
                            usable,
                            placements,
                            courtyard_lookup,
                            courtyard_offset_lookup,
                        )
                    })
                })
                .copied()
                .map(|cand| {
                    let rotation = rotations
                        .into_iter()
                        .find(|rot| {
                            slot_is_legal(
                                cand,
                                *rot,
                                comp_id,
                                board,
                                unrot_half_w,
                                unrot_half_h,
                                passive_offset,
                                usable,
                                placements,
                                courtyard_lookup,
                                courtyard_offset_lookup,
                            )
                        })
                        .unwrap_or(Rotation::Zero);
                    (cand, rotation)
                })
                .filter(|(cand, _)| {
                    let dx = cand.x_nm - aim_point.x_nm;
                    let dy = cand.y_nm - aim_point.y_nm;
                    dx * dx + dy * dy <= BOUND_MAX_AIM_DISTANCE_NM * BOUND_MAX_AIM_DISTANCE_NM
                })
        });

        let candidates = if aim_choice.is_some() {
            Vec::new()
        } else {
            hull.sample_boundary_candidates(target_point, usable, unrot_half_w, unrot_half_h)
        };
        let rotations = [
            Rotation::Zero,
            Rotation::Ninety,
            Rotation::OneEighty,
            Rotation::TwoSeventy,
        ];

        let mut best_choice: Option<(Point, Rotation, i64)> =
            aim_choice.map(|(cand, rotation)| (cand, rotation, 0));

        for cand in &candidates {
            for &rot in &rotations {
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
                    if std::env::var("SYNTH_DEBUG_AIM").is_ok() && pad_aim.is_some() {
                        eprintln!("    reject: intersects placed");
                    }
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
            hull.add_center_for(
                comp_id,
                crate::courtyard_rect_for_placement(
                    &placement,
                    w_mm,
                    h_mm,
                    Some(passive_offset),
                    0,
                ),
            );
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
            hull.add_center_for(
                comp_id,
                crate::courtyard_rect_for_placement(&fallback, w_mm, h_mm, Some(passive_offset), 0),
            );
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

/// Endpoint count above which a net is treated as a distribution rail rather
/// than a local connection when choosing where to pack a passive.
const RAIL_FANOUT: usize = 8;

fn compute_target_centroid(
    board: &Board,
    nets: &[NetId],
    net_pad_positions: &HashMap<NetId, Vec<Point>>,
) -> Option<Point> {
    // A rail's pads are spread across the whole board, so letting it pull on
    // the centroid drags a part toward the middle of the board. Most designs
    // leave nets unnamed (`net_0`, ...), so names alone miss the rails:
    // also recognise declared power nets, ground by pin name, and
    // high-fanout nets (the same structural test the router uses for planes).
    let is_global_rail = |net_id: NetId| -> bool {
        let Some(net) = board.nets.iter().find(|n| n.id == net_id) else {
            return false;
        };
        let name = net.name.to_lowercase();
        let named_rail = matches!(name.as_str(), "gnd" | "vcc" | "vdd" | "3v3" | "5v" | "vbus");
        let ground_pins = net.endpoints.iter().any(|ep| {
            board
                .component(ep.component)
                .and_then(|c| c.part.as_ref())
                .and_then(|p| p.pins.get(ep.pin.0 as usize))
                .is_some_and(|pin| {
                    let pin_name = pin.name.to_lowercase();
                    pin_name.contains("gnd") || pin_name.contains("vss")
                })
        });
        named_rail || net.voltage.is_some() || ground_pins || net.endpoints.len() > RAIL_FANOUT
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
