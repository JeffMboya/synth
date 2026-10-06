// SPDX-License-Identifier: Apache-2.0

//! Functional module extraction for human-quality PCB placement.
//!
//! Groups ICs, power supplies, crystals, and connectors into composite
//! sub-blocks with relative local offsets (dx, dy). When placing an anchor
//! component, its associated passives land in tight relative positions
//! around the anchor rather than floating to distant top-row grid cells.

use std::collections::{HashMap, HashSet};
use synth_geometry::{mm_to_nm, Point, Rotation};
use synth_ir::{Board, ComponentId};

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

/// Orientation of the `index`-th member around its anchor.
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

/// A child component bound to a module anchor with a relative nanometer offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleMember {
    pub id: ComponentId,
    pub offset_nm: Point,
    pub rotation: Rotation,
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
            members.push(ModuleMember {
                id: member.component,
                offset_nm: Point::new(mm_to_nm(dx_mm), mm_to_nm(dy_mm)),
                rotation: module_rotation(cluster.kind, index),
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
                            });
                        } else {
                            modules.push(FunctionalModule {
                                anchor_id: ic.id,
                                members: vec![ModuleMember {
                                    id: component.id,
                                    offset_nm: Point::new(mm_to_nm(dx_mm), mm_to_nm(dy_mm)),
                                    rotation: Rotation::Zero,
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
