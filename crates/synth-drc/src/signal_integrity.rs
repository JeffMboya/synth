// SPDX-License-Identifier: Apache-2.0

//! Signal-integrity rules that read the **routed** copper, not the declared
//! geometry.
//!
//! `synth-validate` already checks a design's declared trace geometry against
//! an impedance target at compile time (`E-SYNTH-IMPEDANCE-*`). These rules are
//! the independent, post-route counterpart: they take whatever copper the
//! router actually produced — re-derived from the `Routing` shapes, never from
//! the router's own statistics — and check it against the same physical
//! models. A trace the exporter derived but the router ignored is exactly the
//! failure this catches.
//!
//! Both rules only fire on a design that asked for the constraint: an
//! impedance rule needs an `impedance` target and a stackup, a skew rule needs
//! a declared `max_skew`. A design that declares neither is unaffected, so the
//! release gate's zero-violation bar is unchanged for boards that never opted
//! in.

use std::collections::BTreeMap;

use synth_geometry::{nm_to_mm, Layer, StackupProfile};
use synth_ir::{Board, Length, NetId};
use synth_pcb::Routing;

use crate::profile::ManufacturerProfile;
use crate::Violation;

/// Fractional deviation past which a routed impedance is a blocking error.
///
/// Matches the compile-time `E-SYNTH-IMPEDANCE-001` threshold. There is no
/// warning tier here because a [`Violation`] carries no severity: any
/// violation counts against the zero-DRC gate, so a rule must only fire when
/// the copper is genuinely wrong.
const ERROR_DEVIATION: f64 = 0.20;

/// The routed width and layer that carry the most copper for `net`.
///
/// A net's impedance is set by the trace that dominates it, so the longest
/// total length wins; ties break toward the lower layer index then the
/// narrower width, so the choice is deterministic for a given routing.
fn dominant_routing(routing: &Routing, net: NetId) -> Option<(Layer, i64)> {
    let mut by_key: BTreeMap<(Layer, i64), i64> = BTreeMap::new();
    for segment in routing.segments.iter().filter(|s| s.net == net) {
        let length_nm = (segment.end.x_nm - segment.start.x_nm).abs()
            + (segment.end.y_nm - segment.start.y_nm).abs();
        *by_key.entry((segment.layer, segment.width_nm)).or_default() += length_nm;
    }
    let mut entries: Vec<((Layer, i64), i64)> = by_key.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    entries.first().map(|(key, _)| *key)
}

fn segments_for_net(routing: &Routing, net: NetId) -> Vec<synth_geometry::Point> {
    routing
        .segments
        .iter()
        .filter(|s| s.net == net)
        .flat_map(|s| [s.start, s.end])
        .collect()
}

/// Controlled-impedance copper check. Code: `E-SYNTH-DRC-012`.
///
/// For every `diff_pair` with an `impedance` target, estimate the impedance
/// the **routed** trace width gives on the layer it was routed on and compare
/// it with the target. Single-ended targets (a pair whose two halves are the
/// same net) use the microstrip/stripline model for the routed layer; a true
/// pair uses the coupled-microstrip model and the declared (or manufacturer
/// minimum) gap. An inner-layer pair returns no estimate from the coupled
/// model and is skipped rather than guessed at.
///
/// A leg with no copper is skipped: an unrouted net is `E-SYNTH-ROUTE-001`'s
/// concern, not this rule's.
#[must_use]
pub fn check_controlled_impedance(
    board: &Board,
    routing: &Routing,
    profile: &ManufacturerProfile,
    stackup: &StackupProfile,
) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (positive, negative, _, impedance) in board.impedance_legs() {
        let target = impedance.to_ohms();
        let Some((layer, width_nm)) = dominant_routing(routing, positive.id) else {
            continue;
        };
        let is_pair = positive.id != negative.id;
        if is_pair && dominant_routing(routing, negative.id).is_none() {
            continue;
        }

        let actual = if is_pair {
            let gap_nm = board
                .declared_clearance(positive)
                .map_or(profile.min_copper_clearance_nm, |l| l.0);
            stackup.coupled_zdiff(layer, width_nm, gap_nm)
        } else {
            stackup.z0(layer, width_nm).map(|r| r.z0_ohms)
        };
        let Some(actual) = actual else {
            continue;
        };

        let deviation = (actual - target).abs() / target.max(1.0);
        if deviation <= ERROR_DEVIATION {
            continue;
        }

        let subject = if is_pair {
            format!("pair `{}`/`{}`", positive.name, negative.name)
        } else {
            format!("net `{}`", positive.name)
        };
        let expected = if is_pair {
            stackup
                .coupled_for_z0(
                    layer,
                    target,
                    profile.min_trace_width_nm,
                    profile.min_copper_clearance_nm,
                )
                .ok()
                .map(|g| format!("about {:.3} mm wide", g.width_mm))
        } else {
            stackup
                .width_for_z0(layer, target, profile.min_trace_width_nm)
                .ok()
                .map(|w| format!("about {:.3} mm wide", w.to_mm()))
        };
        let mut nets = vec![positive.id];
        if is_pair {
            nets.push(negative.id);
        }
        violations.push(Violation {
            code: "E-SYNTH-DRC-012".to_string(),
            message: format!(
                "{subject} is routed {:.3} mm wide on {}; that gives {actual:.1} ohm, \
                 {:.0} percent off the {target:.1} ohm target{}",
                nm_to_mm(width_nm),
                layer.name(),
                deviation * 100.0,
                expected.map_or(String::new(), |e| format!(
                    " (the target needs {e} on this stackup)"
                )),
            ),
            witness: segments_for_net(routing, positive.id),
            nets,
            components: Vec::new(),
            pos_mm: dominant_routing(routing, positive.id).and_then(|_| {
                routing
                    .segments
                    .iter()
                    .find(|s| s.net == positive.id)
                    .map(|s| (nm_to_mm(s.start.x_nm), nm_to_mm(s.start.y_nm)))
            }),
            suggested_override: None,
        });
    }
    violations
}

/// Differential-pair length-match check. Code: `E-SYNTH-DRC-014`.
///
/// Sums the routed copper on each half of every `diff_pair` that declares a
/// `max_skew` and flags a pair whose halves differ by more than that. Computed
/// from the emitted segments, so it holds regardless of what a router reported
/// about its own length matching. A half with no copper is skipped.
#[must_use]
pub fn check_diff_pair_skew(board: &Board, routing: &Routing) -> Vec<Violation> {
    let mut violations = Vec::new();
    for pair in &board.diff_pairs {
        let Some(max_skew) = pair.max_skew else {
            continue;
        };
        let [(_, Some(positive)), (_, Some(negative))] = board.diff_pair_legs(pair) else {
            continue;
        };
        if positive.id == negative.id {
            continue;
        }
        let positive_len = routing.length_for_net(positive.id);
        let negative_len = routing.length_for_net(negative.id);
        if positive_len == 0 || negative_len == 0 {
            continue;
        }
        let skew = (positive_len - negative_len).abs();
        if skew <= max_skew.0 {
            continue;
        }
        violations.push(Violation {
            code: "E-SYNTH-DRC-014".to_string(),
            message: format!(
                "pair `{}`/`{}` is length-matched to {:.3} mm, longer than the {:.3} mm \
                 maximum skew",
                positive.name,
                negative.name,
                nm_to_mm(skew),
                Length(max_skew.0).to_mm(),
            ),
            witness: Vec::new(),
            nets: vec![positive.id, negative.id],
            components: Vec::new(),
            pos_mm: None,
            suggested_override: None,
        });
    }
    violations
}
