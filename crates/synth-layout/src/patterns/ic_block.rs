// SPDX-License-Identifier: Apache-2.0
//! Pass 6: IC + decoupling caps + reset network passives + pull-up/down resistors.

use std::collections::{HashMap, HashSet};
use synth_ir::{Board, ComponentId};

use super::Pattern;
use crate::{
    classify_ic_pin_layout, compute_anchor_pin_offset, Cluster, ClusterKind, ClusterMember,
    MemberSide, PinSide,
};

pub(crate) struct IcBlock;

impl Pattern for IcBlock {
    fn recognize(board: &Board, claimed: &mut HashSet<ComponentId>) -> Vec<Cluster> {
        let mut clusters: Vec<Cluster> = Vec::new();
        let mut anchor_index: HashMap<ComponentId, usize> = HashMap::new();
        for component in &board.components {
            if claimed.contains(&component.id) {
                continue;
            }
            let Some(part) = component.part.as_ref() else {
                continue;
            };
            let has_decoupling = !part.required_decoupling.is_empty();
            let has_reset_pin = part.pins.iter().any(|p| {
                use synth_registry::PinCapability;
                p.capabilities.contains(&PinCapability::Reset)
            });
            if !has_decoupling && !has_reset_pin {
                continue;
            }

            let mut members: Vec<ClusterMember> = Vec::new();

            // Decoupling caps, one per `decoupling.count` per pin.
            // Marked as `Below`: they hang under the IC body.
            for decoupling in &part.required_decoupling {
                let Some(pin_idx) = part.pins.iter().position(|p| p.name == decoupling.net) else {
                    continue;
                };
                let pid = synth_ir::PinId(pin_idx as u32);
                let max_claims = decoupling.count as usize;
                let mut claims_for_pin = 0_usize;
                'cap_outer: for (_net_id, net) in board.nets_containing(component.id, pid) {
                    for endpoint in &net.endpoints {
                        if claims_for_pin >= max_claims {
                            break 'cap_outer;
                        }
                        if endpoint.component == component.id
                            || claimed.contains(&endpoint.component)
                        {
                            continue;
                        }
                        let Some(other) = board.component(endpoint.component) else {
                            continue;
                        };
                        let Some(other_part) = other.part.as_ref() else {
                            continue;
                        };
                        if other_part.kind == "capacitor" {
                            members.push(ClusterMember {
                                id: endpoint.component,
                                side: MemberSide::Below,
                            });
                            claimed.insert(endpoint.component);
                            claims_for_pin += 1;
                        }
                    }
                }
            }

            // Reset network: passives on a reset-capability pin's net.
            // The passives sit on whichever side the reset pin actually
            // occupies — the stock KiCad symbol's geometry when the part
            // has one (STM32F103's NRST is on the LEFT), otherwise the
            // synthesized-model convention (Reset → Right; see
            // classify_ic_pin in synth-web). Wires from the reset pin go
            // straight out without curling around the body.
            for (pin_idx, pin) in part.pins.iter().enumerate() {
                use synth_registry::PinCapability;
                if !pin.capabilities.contains(&PinCapability::Reset) {
                    continue;
                }
                let member_side = match compute_anchor_pin_offset(part, pin_idx).2 {
                    PinSide::Left => MemberSide::Left,
                    _ => MemberSide::Right,
                };
                let pid = synth_ir::PinId(pin_idx as u32);
                for (_net_id, net) in board.nets_containing(component.id, pid) {
                    for endpoint in &net.endpoints {
                        if endpoint.component == component.id
                            || claimed.contains(&endpoint.component)
                        {
                            continue;
                        }
                        let Some(other) = board.component(endpoint.component) else {
                            continue;
                        };
                        let Some(other_part) = other.part.as_ref() else {
                            continue;
                        };
                        if matches!(
                            other_part.kind.as_str(),
                            "resistor" | "capacitor" | "switch"
                        ) {
                            members.push(ClusterMember {
                                id: endpoint.component,
                                side: member_side,
                            });
                            claimed.insert(endpoint.component);
                        }
                    }
                }
            }

            // Pull-up / pull-down resistors: resistors connected to a signal pin of the IC
            // where the other pin is tied to a power rail (VCC/GND/etc.) or another power/ground pin
            for (pin_idx, pin) in part.pins.iter().enumerate() {
                let side = classify_ic_pin_layout(pin);
                // Ignore power pins (top/bottom)
                if side == PinSide::Top || side == PinSide::Bottom {
                    continue;
                }
                let pid = synth_ir::PinId(pin_idx as u32);
                for (_net_id, net) in board.nets_containing(component.id, pid) {
                    for endpoint in &net.endpoints {
                        if endpoint.component == component.id
                            || claimed.contains(&endpoint.component)
                        {
                            continue;
                        }
                        let Some(other) = board.component(endpoint.component) else {
                            continue;
                        };
                        let Some(other_part) = other.part.as_ref() else {
                            continue;
                        };
                        if other_part.kind != "resistor" {
                            continue;
                        }

                        // Check if the other pin of this resistor connects to power
                        let mut is_pull = false;
                        for other_pin_idx in 0..2 {
                            let other_pid = synth_ir::PinId(other_pin_idx as u32);
                            for (_other_net_id, other_net) in
                                board.nets_containing(other.id, other_pid)
                            {
                                for other_ep in &other_net.endpoints {
                                    if let Some(comp) = board.component(other_ep.component) {
                                        if let Some(p) = comp.part.as_ref() {
                                            if let Some(pin) = p.pins.get(other_ep.pin.0 as usize) {
                                                let pin_name_lower = pin.name.to_lowercase();
                                                if matches!(
                                                    pin_name_lower.as_str(),
                                                    "vcc"
                                                        | "vdd"
                                                        | "gnd"
                                                        | "vss"
                                                        | "vbus"
                                                        | "vin"
                                                        | "vout"
                                                        | "3v3"
                                                        | "5v"
                                                ) {
                                                    is_pull = true;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if is_pull {
                            // Pull resistors conventionally stand above
                            // the driven logic pin, rising toward their
                            // supply rail. Keeping them out of the pin
                            // column also leaves bus labels readable.
                            let member_side = MemberSide::Above;
                            members.push(ClusterMember {
                                id: other.id,
                                side: member_side,
                            });
                            claimed.insert(other.id);
                        }
                    }
                }
            }

            claimed.insert(component.id);
            members.sort_by_key(|m| m.id.0);
            anchor_index.insert(component.id, clusters.len());
            clusters.push(Cluster {
                kind: ClusterKind::IcBlock,
                anchor: component.id,
                anchor_vertical: false,
                members,
            });
        }
        clusters
    }
}

/// Whether a pin name denotes a ground reference (`gnd`, `vss`, …).
fn is_ground_pin_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "gnd" | "vss" | "vssa" | "gnda" | "gnd_a" | "vee" | "vneg" | "agnd" | "dgnd" | "ground"
    ) || lower.starts_with("gnd")
        || lower.starts_with("vss")
}

/// Whether a net is a supply rail: it carries a power-output pin, or a
/// non-ground power-input pin (a regulator input, an MCU `vdd`).
fn net_is_rail(board: &Board, net: &synth_ir::Net) -> bool {
    use synth_registry::ElectricalType;
    net.endpoints.iter().any(|ep| {
        board.pin(ep.component, ep.pin).is_some_and(|p| {
            matches!(p.electrical_type, ElectricalType::PowerOutput)
                || (matches!(p.electrical_type, ElectricalType::PowerInput)
                    && !is_ground_pin_name(&p.name))
        })
    })
}

/// Whether a net is a ground: it touches a ground-named power pin.
fn net_is_ground(board: &Board, net: &synth_ir::Net) -> bool {
    use synth_registry::ElectricalType;
    net.endpoints.iter().any(|ep| {
        board.pin(ep.component, ep.pin).is_some_and(|p| {
            matches!(
                p.electrical_type,
                ElectricalType::PowerInput | ElectricalType::GroundReference
            ) && is_ground_pin_name(&p.name)
        })
    })
}

/// Ranking key for choosing which cluster adopts an orphan rail cap.
///
/// Ordering, best first: the anchor with the most pins on the shared
/// rail, then the one nearest in declaration order, then the
/// earliest-declared anchor. The last term is what makes the order
/// *total*.
///
/// It has to be total. `attach_orphan_rail_caps` scans a `HashMap`,
/// and Rust does not guarantee iteration order — so a key that tied
/// on the first two terms would be broken by whichever pair the map
/// happened to yield first, moving a decoupling cap between regions
/// depending on the process's hash seed. Negating the last two keeps
/// the whole tuple ascending-best-first under plain `>` comparison.
type OrphanAnchorRank = (usize, usize, u32);

fn orphan_anchor_rank(
    pins_on_rail: usize,
    anchor: ComponentId,
    cap: ComponentId,
) -> OrphanAnchorRank {
    let adjacency = (anchor.0 as i64 - cap.0 as i64).unsigned_abs() as usize;
    (pins_on_rail, usize::MAX - adjacency, u32::MAX - anchor.0)
}

/// Second sweep (schematic-quality plan Phase A2): a two-pin capacitor
/// whose pins join a rail and a ground, but which no earlier pass
/// claimed, belongs to its heaviest rail consumer's `IcBlock`.
///
/// The claim rule above only reaches caps through the anchor's own
/// `required_decoupling` nets with a per-pin count cap. On a shared
/// rail (one merged net feeding the regulator, the MCU and the
/// sensor) the leftover caps fell through to `Singleton` and were
/// placed by power-flow layer — 150+ mm from the IC they decouple.
/// Here each orphan claims the anchor with the most pins on its rail
/// net (the part drawing most from that rail), tie-broken by
/// declaration adjacency, which is how authors already express
/// intent (`C4` sits next to `U2` in the source).
pub(crate) fn attach_orphan_rail_caps(
    board: &Board,
    claimed: &mut HashSet<ComponentId>,
    clusters: &mut [Cluster],
) {
    // Any already-formed cluster may adopt an orphan — an LDO's bulk
    // and output caps hang off an `LdoBlock`, not an `IcBlock`.
    // Passive-anchored clusters (a divider's resistor, an LED's
    // series R) are excluded: they consume no rail and would drag
    // caps away from the part that does.
    let anchor_index: HashMap<ComponentId, usize> = clusters
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            board
                .component(c.anchor)
                .and_then(|comp| comp.part.as_ref())
                .is_some_and(|part| {
                    !matches!(part.kind.as_str(), "capacitor" | "resistor" | "inductor")
                })
        })
        .map(|(idx, c)| (c.anchor, idx))
        .collect();
    for component in &board.components {
        if claimed.contains(&component.id) {
            continue;
        }
        let Some(part) = component.part.as_ref() else {
            continue;
        };
        if part.kind != "capacitor" || part.pins.len() != 2 {
            continue;
        }
        let nets: Vec<&synth_ir::Net> = (0..2)
            .filter_map(|i| {
                board
                    // u32 cast is bounded: two-pin parts have indexes 0 and 1.
                    .nets_containing(component.id, synth_ir::PinId(i as u32))
                    .map(|(_, net)| net)
                    .next()
            })
            .collect();
        if nets.len() != 2 || nets[0].id == nets[1].id {
            continue;
        }
        let rail = if net_is_rail(board, nets[0]) && net_is_ground(board, nets[1]) {
            nets[0]
        } else if net_is_rail(board, nets[1]) && net_is_ground(board, nets[0]) {
            nets[1]
        } else {
            continue;
        };
        // Heaviest consumer wins; declaration adjacency breaks ties
        // (lower adjacency distance = nearer in source = preferred),
        // and the anchor's own id breaks a remaining exact tie so the
        // choice never depends on `HashMap` iteration order.
        let mut best: Option<(usize, usize, u32, usize)> = None;
        for (&anchor, &cluster_idx) in &anchor_index {
            if anchor == component.id {
                continue;
            }
            // A declared `group` is a hard boundary: adopting across
            // one would place the cap in another group's region.
            if board.component(anchor).and_then(|c| c.group.as_deref())
                != component.group.as_deref()
            {
                continue;
            }
            let pins_on_rail = rail
                .endpoints
                .iter()
                .filter(|ep| ep.component == anchor)
                .count();
            if pins_on_rail == 0 {
                continue;
            }
            let key = orphan_anchor_rank(pins_on_rail, anchor, component.id);
            if best.is_none_or(|(best_pins, best_adj, best_anchor, _)| {
                key > (best_pins, best_adj, best_anchor)
            }) {
                best = Some((key.0, key.1, key.2, cluster_idx));
            }
        }
        let Some((_, _, _, cluster_idx)) = best else {
            continue;
        };
        clusters[cluster_idx].members.push(ClusterMember {
            id: component.id,
            side: MemberSide::Below,
        });
        clusters[cluster_idx].members.sort_by_key(|m| m.id.0);
        claimed.insert(component.id);
    }
}

#[cfg(test)]
mod tests {
    use super::{orphan_anchor_rank, OrphanAnchorRank};
    use synth_ir::ComponentId;

    const CAP: ComponentId = ComponentId(10);

    fn rank(pins: usize, anchor: u32) -> OrphanAnchorRank {
        orphan_anchor_rank(pins, ComponentId(anchor), CAP)
    }

    /// The anchor drawing most from the shared rail wins.
    #[test]
    fn heaviest_rail_consumer_wins() {
        assert!(rank(3, 1) > rank(2, 2));
        assert!(rank(2, 1) > rank(1, 99));
    }

    /// With pins equal, the anchor nearest in declaration order wins —
    /// either side of the cap.
    #[test]
    fn nearer_in_declaration_order_wins() {
        assert!(rank(1, 11) > rank(1, 20)); // cap at 10: 11 is nearer than 20
        assert!(rank(1, 9) > rank(1, 0)); // …and on the low side too
    }

    /// The regression: two anchors equidistant from the cap, drawing the
    /// same number of pins from the rail. This is the exact case that
    /// used to be decided by `HashMap` iteration order, so the cap
    /// could land in different regions on different runs. The rank
    /// must be decided by the anchor id alone, and must not compare
    /// equal — otherwise the selection reverts to "first yielded".
    #[test]
    fn equidistant_equal_pins_anchors_are_ordered_by_id_not_hash_order() {
        let a = rank(2, 8);
        let b = rank(2, 12);
        assert_ne!(a, b, "an exact tie would be broken by map order");
        assert!(a > b, "the earlier-declared anchor wins the tie");

        // And it is the same whichever order the two are presented in.
        assert_eq!(a.max(b), a);
        assert_eq!(b.max(a), a);
    }

    /// Totality across a spread of anchors: no two distinct anchors may
    /// ever produce equal ranks, for any pin count.
    #[test]
    fn rank_is_total_over_distinct_anchors() {
        for pins in 0..4 {
            let mut seen = std::collections::HashSet::new();
            for anchor in 0..32u32 {
                assert!(
                    seen.insert(rank(pins, anchor)),
                    "pins={pins} anchor={anchor} collides with another anchor's rank"
                );
            }
        }
    }
}
