// SPDX-License-Identifier: Apache-2.0

//! Repeated-circuit detection for the placement review.
//!
//! Two output drivers, three identical input stages, four LED rows: a board
//! often holds several copies of one small circuit, and a reader expects the
//! copies to be laid out alike. This module finds them by structure alone,
//! with no part named: the board is cut at the large ICs and at the supply
//! rails, the small parts that remain and share a signal net form an
//! *island*, and each island gets a fingerprint from its part types and its
//! internal wiring. Islands with equal fingerprints are copies.
//!
//! What it decides is membership and the part-for-part correspondence between
//! two copies. Where the parts sit is the caller's business.

use std::collections::{HashMap, HashSet};

use synth_ir::{Board, ComponentId};

/// A part with more pins than this is a hub (an MCU, a regulator, a sensor): the
/// board is cut there, so circuits wired to one hub do not merge into one.
const HUB_PINS: usize = 5;

/// Nets with more endpoints than this are distribution rails, not signals.
const RAIL_FANOUT: usize = 8;

/// Colour-refinement rounds. Enough to tell apart circuits a few parts across.
const ROUNDS: usize = 4;

/// One copy of a repeated circuit.
#[derive(Debug, Clone)]
pub(crate) struct Island {
    /// The part the copy is positioned from: its connector, else its largest part.
    pub anchor: ComponentId,
    /// Every part in the copy, with a label that matches across copies.
    pub parts: Vec<(ComponentId, u64)>,
}

fn hash_of<T: std::hash::Hash>(value: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn is_hub(board: &Board, id: ComponentId) -> bool {
    board
        .component(id)
        .and_then(|c| c.part.as_ref())
        .is_some_and(|part| part.pins.len() > HUB_PINS)
}

pub(crate) fn is_rail(board: &Board, net: &synth_ir::Net) -> bool {
    let name = net.name.to_lowercase();
    let named = matches!(
        name.as_str(),
        "gnd" | "vcc" | "vdd" | "3v3" | "5v" | "vbus" | "agnd" | "pgnd"
    );
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
    named || net.voltage.is_some() || ground_pins || net.endpoints.len() > RAIL_FANOUT
}

fn pin_name(board: &Board, endpoint: &synth_ir::NetEndpoint) -> String {
    board
        .component(endpoint.component)
        .and_then(|c| c.part.as_ref())
        .and_then(|p| p.pins.get(endpoint.pin.0 as usize))
        .map_or_else(|| endpoint.pin.0.to_string(), |pin| pin.name.clone())
}

/// Union-find root, with path halving.
fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// Every group of two or more islands with the same fingerprint, each group
/// ordered by its first part's refdes so the first copy is the reference.
pub(crate) fn find_repeated_circuits(board: &Board) -> Vec<Vec<Island>> {
    // Small parts, linked through non-rail nets.
    let small: Vec<ComponentId> = board
        .components
        .iter()
        .filter(|c| c.part.is_some() && !is_hub(board, c.id))
        .map(|c| c.id)
        .collect();
    let index: HashMap<ComponentId, usize> =
        small.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut parent: Vec<usize> = (0..small.len()).collect();
    let signal_nets: Vec<&synth_ir::Net> = board
        .nets
        .iter()
        .filter(|net| !is_rail(board, net))
        .collect();
    for net in &signal_nets {
        let members: Vec<usize> = net
            .endpoints
            .iter()
            .filter_map(|ep| index.get(&ep.component).copied())
            .collect();
        for pair in members.windows(2) {
            let (a, b) = (find(&mut parent, pair[0]), find(&mut parent, pair[1]));
            if a != b {
                parent[a] = b;
            }
        }
    }
    let mut islands: HashMap<usize, Vec<ComponentId>> = HashMap::new();
    for (i, id) in small.iter().enumerate() {
        let root = find(&mut parent, i);
        islands.entry(root).or_default().push(*id);
    }
    let mut islands: Vec<Vec<ComponentId>> = islands
        .into_values()
        .filter(|parts| parts.len() >= 2)
        .collect();
    for parts in &mut islands {
        parts.sort_by_key(|id| id.0);
    }
    islands.sort_by_key(|parts| parts[0].0);

    let mut by_fingerprint: HashMap<u64, Vec<Island>> = HashMap::new();
    let mut order: Vec<u64> = Vec::new();
    for parts in islands {
        let island: HashSet<ComponentId> = parts.iter().copied().collect();
        let mut label: HashMap<ComponentId, u64> = parts
            .iter()
            .map(|id| {
                let part = board
                    .component(*id)
                    .and_then(|c| c.part.as_ref())
                    .map_or(String::new(), |p| p.id.0.clone());
                (*id, hash_of(&part))
            })
            .collect();
        // The nets that touch the island, and how each leaves it.
        let touching: Vec<&synth_ir::Net> = board
            .nets
            .iter()
            .filter(|net| {
                net.endpoints
                    .iter()
                    .any(|ep| island.contains(&ep.component))
            })
            .collect();
        for _ in 0..ROUNDS {
            let net_label: Vec<u64> = touching
                .iter()
                .map(|net| {
                    let mut inside: Vec<(String, u64)> = net
                        .endpoints
                        .iter()
                        .filter(|ep| island.contains(&ep.component))
                        .map(|ep| (pin_name(board, ep), label[&ep.component]))
                        .collect();
                    inside.sort();
                    // Leaving the island by a rail is one thing, by a signal
                    // another; which rail or which hub pin is not part of the
                    // circuit.
                    let leaves = u8::from(is_rail(board, net));
                    let outside = net
                        .endpoints
                        .iter()
                        .filter(|ep| !island.contains(&ep.component))
                        .count()
                        .min(1);
                    hash_of(&(inside, leaves, outside))
                })
                .collect();
            let mut next = label.clone();
            for id in &parts {
                let mut around: Vec<(String, u64)> = Vec::new();
                for (net, nl) in touching.iter().zip(&net_label) {
                    for ep in net.endpoints.iter().filter(|ep| ep.component == *id) {
                        around.push((pin_name(board, ep), *nl));
                    }
                }
                around.sort();
                next.insert(*id, hash_of(&(label[id], around)));
            }
            label = next;
        }
        let mut sorted: Vec<u64> = label.values().copied().collect();
        sorted.sort_unstable();
        let fingerprint = hash_of(&sorted);
        // Anchor: a connector if there is one, else the part with the most pins.
        let anchor = parts
            .iter()
            .copied()
            .max_by_key(|id| {
                let c = board.component(*id);
                let connector = c.is_some_and(|c| matches!(c.kind.as_str(), "connector" | "jack"));
                let pins = c.and_then(|c| c.part.as_ref()).map_or(0, |p| p.pins.len());
                (u8::from(connector), pins, std::cmp::Reverse(id.0))
            })
            .unwrap_or(parts[0]);
        by_fingerprint
            .entry(fingerprint)
            .or_insert_with(|| {
                order.push(fingerprint);
                Vec::new()
            })
            .push(Island {
                anchor,
                parts: parts.iter().map(|id| (*id, label[id])).collect(),
            });
    }
    order
        .into_iter()
        .filter_map(|fp| by_fingerprint.remove(&fp))
        .filter(|group| group.len() >= 2)
        .collect()
}

impl Island {
    /// The part of this copy that corresponds to `label` in another copy, when
    /// exactly one part carries it.
    pub(crate) fn part_with_label(&self, label: u64) -> Option<ComponentId> {
        let mut found = self.parts.iter().filter(|(_, l)| *l == label);
        let first = found.next()?;
        if found.next().is_some() {
            None
        } else {
            Some(first.0)
        }
    }
}
