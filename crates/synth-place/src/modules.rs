// SPDX-License-Identifier: Apache-2.0

//! Functional module extraction for human-quality PCB placement.
//!
//! Groups ICs, power supplies, crystals, and connectors into composite
//! sub-blocks. When placing an anchor component, its associated passives
//! land beside it rather than floating to distant top-row grid cells —
//! beside the *pad* it binds to, not merely near the chip.
//!
//! Two things decide where a member goes:
//!
//! 1. Its [`MemberBinding`] — the net and the two pins that tie it to its
//!    anchor. A decoupling capacitor is placed against the rail pad it
//!    decouples and turned so that pad faces the anchor, so the connection
//!    is a short direct run rather than a detour around the body.
//! 2. A fallback offset from the anchor's centre, used when the anchor's
//!    pad geometry is unavailable (a part with no resolved footprint
//!    carries no pad offsets, and there is then no pad to aim at).
//!
//! The binding decides *where to aim*; the placement search still resolves
//! the exact legal grid slot, so a pad-aware target can never make a board
//! unplaceable.

use std::collections::{HashMap, HashSet};
use synth_geometry::{mm_to_nm, Point, Rect, Rotation};
use synth_ir::{Board, ComponentId, MemberBinding, PinId};

/// Stand-off between an anchor's pad and the member sitting beside it.
///
/// Matches the gap the hard-`near` hint path has always used for
/// pad-adjacent placement, so a module member and a hinted part behave the
/// same way.
pub const PAD_STANDOFF_MM: f64 = 1.0;

/// Gap kept between two members stacked along the same pad normal.
pub const MEMBER_STACK_GAP_MM: f64 = 0.5;

/// Whether a recognized motif is bound to its anchor on the board at all.
fn binds_to_board(kind: synth_ir::ClusterKind) -> bool {
    matches!(
        kind,
        synth_ir::ClusterKind::IcBlock
            | synth_ir::ClusterKind::Crystal
            | synth_ir::ClusterKind::UsbEsd
            | synth_ir::ClusterKind::LdoBlock
    )
}

/// Whether a member's role makes it part of its anchor's board module.
fn binds_role(role: synth_ir::MemberRole) -> bool {
    matches!(
        role,
        synth_ir::MemberRole::DecouplingCap
            | synth_ir::MemberRole::RailCap
            | synth_ir::MemberRole::LoadCap
            | synth_ir::MemberRole::EsdDiode
    )
}

/// Orientation of the `index`-th member around its anchor, used only when
/// the member has no pad binding to orient by.
///
/// Decoupling capacitors alternate around the IC so two caps on the same
/// rail do not land on the same spot; every other motif points its
/// members the same way.
fn module_rotation(kind: synth_ir::ClusterKind, index: usize) -> Rotation {
    if kind != synth_ir::ClusterKind::IcBlock {
        return Rotation::Zero;
    }
    match index % 4 {
        0 => Rotation::OneEighty,
        1 => Rotation::Zero,
        2 => Rotation::Ninety,
        _ => Rotation::TwoSeventy,
    }
}

/// Where a member that binds to an anchor pad should sit, and how it
/// should be turned.
///
/// Derived from the binding alone, so it can be computed once the anchor is
/// placed and its rotation is known. `None` when the anchor has no usable
/// pad geometry for this binding, which sends the member down the
/// centre-offset fallback path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PadBoundPlacement {
    pub center: Point,
    pub rotation: Rotation,
    /// Unit vector along the anchor edge the pad faces, which is the
    /// direction a slot stacks in when several members share that edge.
    pub outward: (i64, i64),
    /// Half the member's rotated extent along `outward`, so a caller
    /// stacking slots knows how much room each one takes.
    pub half_along_outward: i64,
}

/// Aim a module member at the anchor pad its binding names.
///
/// The pad decides *which side* of the anchor the member belongs on, and
/// the anchor's courtyard decides *how far out* it stands. That split is
/// deliberate: courtyards may not overlap, and a footprint's courtyard
/// reaches past its own pads — a USB-C receptacle's front pads sit well
/// inside its keep-clear body, and a DIP's courtyard is centred on the part
/// while its placement anchor is at one end. Aiming at the pad coordinate
/// alone therefore produces members that overlap the anchor they belong to;
/// aiming at the courtyard the placer actually enforces cannot.
///
/// The member is rotated so its own bound pad points back at the anchor,
/// which turns the connection into the shortest possible run.
///
/// Several members can share one edge — a rail declared with `count = 4` has
/// four caps to place, and a crystal's two load capacitors bind to two pins
/// on the same edge. Stacking them along the outward normal is the caller's
/// job, once it knows which members resolved to the same side; this returns
/// the base aim for one member and reports the side and half-extent that
/// stacking needs.
///
/// Returns `None` when the pad sits at the anchor's centre, which carries
/// no direction to stand off in.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn pad_bound_placement(
    anchor_center: Point,
    anchor_rotation: Rotation,
    anchor_courtyard: Rect,
    anchor_pad_offset: (i64, i64),
    member_pad_offset: (i64, i64),
    member_far_pad_offset: (i64, i64),
    member_half_extents: (i64, i64),
) -> Option<PadBoundPlacement> {
    let (pad_x, pad_y) = anchor_rotation.rotate_offset(anchor_pad_offset.0, anchor_pad_offset.1);
    let pad = Point::new(anchor_center.x_nm + pad_x, anchor_center.y_nm + pad_y);
    let rect_center_x = (anchor_courtyard.min.x_nm + anchor_courtyard.max.x_nm) / 2;
    let rect_center_y = (anchor_courtyard.min.y_nm + anchor_courtyard.max.y_nm) / 2;
    let from_center_x = pad.x_nm - rect_center_x;
    let from_center_y = pad.y_nm - rect_center_y;
    let (outward_x, outward_y) = if from_center_x.abs() >= from_center_y.abs() {
        (from_center_x.signum(), 0)
    } else {
        (0, from_center_y.signum())
    };
    if outward_x == 0 && outward_y == 0 {
        return None;
    }

    // Turn the member so its bound pad faces back at the anchor. The
    // member's axis runs from its far pad to the pad it binds; the
    // rotation that puts that axis along -outward wins, ties going to the
    // earliest rotation so the choice stays deterministic.
    let axis_x = member_pad_offset.0 - member_far_pad_offset.0;
    let axis_y = member_pad_offset.1 - member_far_pad_offset.1;
    let (want_x, want_y) = (-outward_x, -outward_y);
    let mut rotation = Rotation::Zero;
    let mut best_alignment = i64::MIN;
    for candidate in [
        Rotation::Zero,
        Rotation::Ninety,
        Rotation::OneEighty,
        Rotation::TwoSeventy,
    ] {
        let (rx, ry) = candidate.rotate_offset(axis_x, axis_y);
        let alignment = rx * want_x + ry * want_y;
        if alignment > best_alignment {
            best_alignment = alignment;
            rotation = candidate;
        }
    }

    let (rotated_half_w, rotated_half_h) = match rotation {
        Rotation::Zero | Rotation::OneEighty => member_half_extents,
        Rotation::Ninety | Rotation::TwoSeventy => (member_half_extents.1, member_half_extents.0),
    };
    let member_half_along_normal = if outward_x != 0 {
        rotated_half_w
    } else {
        rotated_half_h
    };

    // Distance from the anchor centre to the member's centre: out past the
    // anchor's courtyard edge by the standoff, plus the member's own half
    // extent, plus its slot if it shares the side with another member.
    let edge_along_normal = if outward_x != 0 {
        if outward_x > 0 {
            anchor_courtyard.max.x_nm
        } else {
            anchor_courtyard.min.x_nm
        }
    } else if outward_y > 0 {
        anchor_courtyard.max.y_nm
    } else {
        anchor_courtyard.min.y_nm
    };
    // Signed distance from the anchor centre out to that edge, along the
    // outward normal.
    let edge_from_center = (edge_along_normal - anchor_center.x_nm) * outward_x
        + (if outward_x != 0 {
            0
        } else {
            edge_along_normal - anchor_center.y_nm
        }) * outward_y;
    let distance = edge_from_center + mm_to_nm(PAD_STANDOFF_MM) + member_half_along_normal;

    Some(PadBoundPlacement {
        center: Point::new(
            anchor_center.x_nm + outward_x * distance,
            anchor_center.y_nm + outward_y * distance,
        ),
        rotation,
        outward: (outward_x, outward_y),
        half_along_outward: member_half_along_normal,
    })
}

/// The member's other pin, given its bound pin.
///
/// A two-pin part's pads straddle its centre, so the pad on the far side
/// is the mirror image; more than two pins has no such thing and the
/// binding cannot be aimed.
#[must_use]
pub fn far_pad_offset(bound_pad_offset: (i64, i64), pad_count: usize) -> Option<(i64, i64)> {
    if pad_count != 2 {
        return None;
    }
    Some((-bound_pad_offset.0, -bound_pad_offset.1))
}

/// A child component bound to a module anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleMember {
    pub id: ComponentId,
    /// Fallback offset from the anchor's centre, used when the anchor has
    /// no pad geometry to aim the member at.
    pub offset_nm: Point,
    /// Fallback rotation for the same reason.
    pub rotation: Rotation,
    /// The pin-to-pin connection tying this member to its anchor, when
    /// the motif was recognized. `None` for the orphan sweep, which binds
    /// a leftover passive by shared net without a recognized motif.
    pub binding: Option<MemberBinding>,
    /// Ordinal among the members sharing this anchor *and* this anchor
    /// pin, so several caps on one rail fan out instead of stacking.
    pub slot: u32,
}

/// A functional module consisting of an anchor component and its relative child passives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionalModule {
    pub anchor_id: ComponentId,
    pub members: Vec<ModuleMember>,
}

fn compute_dynamic_module_offsets(
    anchor_id: ComponentId,
    child_id: ComponentId,
    courtyard_lookup: &HashMap<ComponentId, (f64, f64)>,
    idx: usize,
) -> (f64, f64) {
    let (aw, ah) = courtyard_lookup
        .get(&anchor_id)
        .copied()
        .unwrap_or((8.0, 8.0));
    let (cw, ch) = courtyard_lookup
        .get(&child_id)
        .copied()
        .unwrap_or((2.0, 2.0));

    // Generous clearance margin: 5.5mm for THT/large ICs (>15mm), 3.0mm for standard SMD
    let margin = if ah > 15.0 || aw > 15.0 { 5.5 } else { 3.0 };
    let dx = aw / 2.0 + cw / 2.0 + margin;
    let dy = ah / 2.0 + ch / 2.0 + margin;

    let patterns = [
        (dx, 0.0),
        (-dx, 0.0),
        (0.0, dy),
        (0.0, -dy),
        (dx, dy / 2.0),
        (-dx, dy / 2.0),
        (dx, -dy / 2.0),
        (-dx, -dy / 2.0),
    ];
    patterns[idx % patterns.len()]
}

/// Extract all functional modules from `board`.
/// Returns the list of modules and the set of component IDs claimed as child members.
///
/// Membership comes from [`synth_ir::recognize_clusters`] — the same
/// recognition pass the schematic layout draws from — so a part the sheet
/// groups with an anchor is a part the board binds beside it. This
/// function only decides *which* recognized members are bound on the
/// board and where they land: a role is a schematic concept (a pull-up
/// rises above its pin on the sheet), and a board module is a
/// placement concept, so the mapping from one to the other is made here.
///
/// [`synth_ir::MemberRole::DecouplingCap`], [`synth_ir::MemberRole::RailCap`],
/// [`synth_ir::MemberRole::LoadCap`] and [`synth_ir::MemberRole::EsdDiode`]
/// are the roles placed beside their anchor. Signal-integrity roles
/// (pull-ups, divider partners, RF matching elements) belong to the
/// schematic only for now: binding them here is measured work, not a
/// refactor, and lands with pad-aware offsets in the next slice.
#[allow(clippy::too_many_lines, clippy::implicit_hasher)]
pub fn extract_functional_modules(
    board: &Board,
    courtyard_lookup: &HashMap<ComponentId, (f64, f64)>,
) -> (Vec<FunctionalModule>, HashSet<ComponentId>) {
    let mut modules = Vec::new();
    let mut claimed_children = HashSet::new();

    // 1-4. Recognized motifs, bound to their anchor.
    //
    // Members arrive in recognition order (net order, then endpoint
    // order) and keep it: the ordinal decides which side of the anchor a
    // member fans out to, so reordering here would move parts on boards
    // that have no reason to move.
    for cluster in synth_ir::recognize_clusters(board) {
        if !binds_to_board(cluster.kind) {
            continue;
        }
        let mut members: Vec<ModuleMember> = Vec::new();
        // Slots are counted per anchor *pin*, not per anchor: an IC with
        // four caps on one rail and one on another fans the four out
        // along that pad without pushing the fifth away from its own.
        let mut slots: HashMap<PinId, u32> = HashMap::new();
        for member in &cluster.members {
            if !binds_role(member.role) {
                continue;
            }
            let index = members.len();
            let (dx_mm, dy_mm) = compute_dynamic_module_offsets(
                cluster.anchor,
                member.component,
                courtyard_lookup,
                index,
            );
            let slot = slots.entry(member.binding.anchor_pin).or_default();
            *slot = slot.saturating_add(1);
            members.push(ModuleMember {
                id: member.component,
                offset_nm: Point::new(mm_to_nm(dx_mm), mm_to_nm(dy_mm)),
                rotation: module_rotation(cluster.kind, index),
                binding: Some(member.binding),
                slot: slot.saturating_sub(1),
            });
            claimed_children.insert(member.component);
        }
        if !members.is_empty() {
            modules.push(FunctionalModule {
                anchor_id: cluster.anchor,
                members,
            });
        }
    }

    // 5. Orphan Passive Net Clustering (bind remaining passives to their net IC anchor)
    for component in &board.components {
        let is_passive = matches!(
            component.kind.as_str(),
            "resistor" | "capacitor" | "inductor" | "diode"
        );
        if !is_passive || claimed_children.contains(&component.id) {
            continue;
        }
        for net in &board.nets {
            let mentions_passive = net.endpoints.iter().any(|ep| ep.component == component.id);
            if !mentions_passive {
                continue;
            }
            for endpoint in &net.endpoints {
                if endpoint.component == component.id
                    || claimed_children.contains(&endpoint.component)
                {
                    continue;
                }
                if let Some(ic) = board.components.iter().find(|c| c.id == endpoint.component) {
                    let is_multi_pin = ic.part.as_ref().is_none_or(|p| p.pins.len() >= 3);
                    if is_multi_pin {
                        let idx = modules
                            .iter()
                            .find(|m| m.anchor_id == ic.id)
                            .map_or(0, |m| m.members.len());
                        let (dx_mm, dy_mm) = compute_dynamic_module_offsets(
                            ic.id,
                            component.id,
                            courtyard_lookup,
                            idx,
                        );

                        if let Some(m) = modules.iter_mut().find(|m| m.anchor_id == ic.id) {
                            m.members.push(ModuleMember {
                                id: component.id,
                                offset_nm: Point::new(mm_to_nm(dx_mm), mm_to_nm(dy_mm)),
                                rotation: Rotation::Zero,
                                binding: None,
                                slot: 0,
                            });
                        } else {
                            modules.push(FunctionalModule {
                                anchor_id: ic.id,
                                members: vec![ModuleMember {
                                    id: component.id,
                                    offset_nm: Point::new(mm_to_nm(dx_mm), mm_to_nm(dy_mm)),
                                    rotation: Rotation::Zero,
                                    binding: None,
                                    slot: 0,
                                }],
                            });
                        }
                        claimed_children.insert(component.id);
                        break;
                    }
                }
            }
            if claimed_children.contains(&component.id) {
                break;
            }
        }
    }

    (modules, claimed_children)
}
