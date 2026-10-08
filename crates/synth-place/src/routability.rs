// SPDX-License-Identifier: Apache-2.0

//! A route-aware estimate of a placement, used to choose between candidates.
//!
//! The placer decides a floorplan from legality and connectivity hints alone:
//! it never asks how the board will actually route. Two placements can be
//! equally legal and equally short-sighted, and the one that routes is the
//! one whose copper is shorter and whose nets are not stretched across the
//! board. This module scores that, cheaply and deterministically, so the
//! export path can prefer the placement that routes more easily without
//! paying for a router run. (Idea borrowed from TraceMaker's `--route-check`;
//! this is its deterministic tier, with the real-route acceptance a separate
//! step.)

use serde::{Deserialize, Serialize};
use synth_geometry::Point;
use synth_ir::{Board, ComponentId};

use crate::Placement;

/// A net this long is treated as a routing hazard rather than a cost.
///
/// Half-perimeter, so it is the span the router must cross, not the length it
/// draws. 40 mm is comfortably longer than any two parts that belong together
/// on the boards Synth targets.
const OVERLONG_NET_NM: i64 = 40 * 1_000_000;

/// A route-aware score for one placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutabilityEstimate {
    /// Sum over nets of the net's half-perimeter bounding box, in nm.
    ///
    /// Total half-perimeter wire length: the standard cheap proxy for how
    /// much copper a route will need. Lower is easier to route.
    pub copper_length_nm: u64,
    /// Nets whose half-perimeter exceeds [`OVERLONG_NET_NM`].
    ///
    /// A handful of long nets is a much worse sign than the same total length
    /// spread evenly, because each one is a congestion event the router has
    /// to solve.
    pub overlong_nets: usize,
}

impl RoutabilityEstimate {
    /// Ordering key: fewer hazards first, then less copper.
    ///
    /// `overlong_nets` leads because one net spanning the board is a routing
    /// failure waiting to happen, whereas a slightly larger total of short
    /// nets routes fine.
    #[must_use]
    pub fn key(&self) -> (usize, u64) {
        (self.overlong_nets, self.copper_length_nm)
    }
}

/// Estimate how easily `placement` routes.
#[must_use]
pub fn routability(board: &Board, placement: &Placement) -> RoutabilityEstimate {
    let centres: std::collections::HashMap<ComponentId, Point> = placement
        .components
        .iter()
        .map(|placed| (placed.id, placed.center))
        .collect();

    let mut copper_length_nm: u64 = 0;
    let mut overlong_nets = 0usize;
    for net in &board.nets {
        let mut min_x = i64::MAX;
        let mut min_y = i64::MAX;
        let mut max_x = i64::MIN;
        let mut max_y = i64::MIN;
        let mut placed_endpoints = 0usize;
        for endpoint in &net.endpoints {
            let Some(point) = centres.get(&endpoint.component) else {
                continue;
            };
            placed_endpoints += 1;
            min_x = min_x.min(point.x_nm);
            min_y = min_y.min(point.y_nm);
            max_x = max_x.max(point.x_nm);
            max_y = max_y.max(point.y_nm);
        }
        if placed_endpoints < 2 {
            continue;
        }
        let span = (max_x - min_x) + (max_y - min_y);
        copper_length_nm += u64::try_from(span.max(0)).unwrap_or(0);
        if span > OVERLONG_NET_NM {
            overlong_nets += 1;
        }
    }

    RoutabilityEstimate {
        copper_length_nm,
        overlong_nets,
    }
}
