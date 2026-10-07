// SPDX-License-Identifier: Apache-2.0

//! Differential pair detection + per-pair routing reports.
//!
//! Plan §10.2 lists diff pairs as the second-highest priority
//! class after RF feeds — they want to be routed first, with
//! their two halves coupled along most of the run, and length-
//! matched within a tolerance.
//!
//! Slice 4 ships the *infrastructure*:
//!
//! - [`resolve_pair_nets`] maps each `board.diff_pairs[i]` to
//!   a `(NetId, NetId)` for the positive / negative halves.
//!   Two resolution strategies: explicit IR net name first
//!   (the canonical user intent), then a fallback that walks
//!   pin capabilities (`UsbDp` / `UsbDn` / `RfFeed` /
//!   differential electrical types) so auto-named nets
//!   produced by lowering still link to the diff_pair
//!   declaration.
//! - Routing-time priority elevation: matched diff-pair nets
//!   get priority class 1 (above the general "all signals
//!   are class 2" default), so they route ahead of every
//!   single-ended net.
//! - [`PairReport`] records per-pair length (nm) and skew so
//!   downstream consumers can decide whether the result meets
//!   the impedance / skew spec the user declared.
//!
//! What slice 4 does *not* ship (deferred to slice 4.x):
//!
//! - **Coupled-lane routing.** The two halves currently route
//!   as independent A* searches; nothing biases the negative
//!   half toward staying parallel to the positive. Coupling
//!   requires a two-pass router that runs the positive first,
//!   then runs the negative with a lower cost on cells
//!   adjacent to the positive's path.
//! - **Length-tuning serpentine.** When skew exceeds the
//!   pair's tolerance, slice 4.x inserts a deterministic
//!   serpentine on the shorter trace to balance.
//! - **`E-SYNTH-ROUTE-DIFF-SKEW` diagnostic.** Slice 6's
//!   catalogue work.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use synth_ir::{Board, NetId};

/// Per-pair length / skew metric attached to a `Routing`. One
/// entry per resolved `DiffPair` in IR declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairReport {
    pub positive: NetId,
    pub negative: NetId,
    /// Total trace length on the positive half, in nm.
    /// `0` when the half wasn't routed (slice 3's max-coverage
    /// fallback may leave a pair half unrouted; consumers
    /// detect that as `length == 0`).
    pub positive_length_nm: i64,
    pub negative_length_nm: i64,
    /// Absolute skew between the two halves, in nm.
    pub skew_nm: i64,
}

/// Resolve every `DiffPair` declared in `board` to `(positive,
/// negative)` net id pairs.
///
/// Resolution lives in [`synth_ir::resolve_pairs`] so the router and the
/// placer agree on which nets form a pair — a capacitor placed "on the
/// pair corridor" is only meaningful if the corridor is the same corridor
/// the router will couple. That pass reads polarity from the pins
/// themselves, so it resolves USB and any other differential pair declared
/// by electrical type, rather than only pairs whose labels contain `dp`.
///
/// Pairs that resolve to nothing are dropped from the returned set (slice 6
/// will promote these to `W-SYNTH-ROUTE-DIFF-UNRESOLVED` warnings).
///
/// Output preserves the IR declaration order of `diff_pairs`.
#[must_use]
pub fn resolve_pair_nets(board: &Board) -> Vec<(NetId, NetId)> {
    synth_ir::resolve_pairs(board)
        .into_iter()
        .map(|pair| (pair.positive_net, pair.negative_net))
        .collect()
}

/// Set of every net id that's part of any resolved diff pair.
/// Used by the router to recognise diff-pair priority without
/// re-running the resolver per net.
#[must_use]
pub fn pair_net_ids(pairs: &[(NetId, NetId)]) -> HashSet<NetId> {
    pairs.iter().flat_map(|(a, b)| [*a, *b]).collect()
}
