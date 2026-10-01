// SPDX-License-Identifier: Apache-2.0

//! `is_two_pin_symbol_kind` is consumed by the schematic *placer*
//! (via `body_size_for_part`, to reserve a body rectangle) and by
//! the schematic *router* (via `pin_terminal_xy`, to compute where a
//! pin's wire stub starts). If the two disagree, the layout reserves
//! one body size and the wires are aimed at terminals drawn for
//! another.
//!
//! There is now a single implementation
//! (`synth_layout::route::is_two_pin_symbol_kind`), so what can still
//! go wrong is the *list*: a `kind` string that matches nothing in
//! the registry (dead weight that reads as coverage), or a registry
//! part that is drawn as a bare two-pin symbol but is missing from
//! the list. Both happened — the list carried `ferrite_bead`,
//! `zener_diode` and `resonator`, none of which are registry `kind`s
//! (ferrite beads are `inductor`, zeners are `diode`), while
//! `inductor` and `switch` were absent, so inductors and 2-pin
//! switches got a multi-pin-sized body.
//!
//! These tests pin the list against the real registry, so adding a
//! new two-pin part without classifying it fails here rather than
//! showing up as a schematic that reserves the wrong box.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use synth_layout::route::is_two_pin_symbol_kind;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .unwrap()
}

fn registry() -> synth_registry::Registry {
    synth_registry::load_dir(&workspace_root().join("registry").join("parts")).unwrap()
}

/// The `kind`s the two-pin list is expected to cover. Kept
/// explicit (rather than reading the list back) so that *removing* a
/// kind from the implementation fails the first test below.
const CLAIMED: &[&str] = &[
    "resistor",
    "capacitor",
    "inductor",
    "diode",
    "led",
    "crystal",
    "switch",
];

#[test]
fn every_claimed_kind_exists_in_the_registry() {
    let registry = registry();
    let present: BTreeSet<&str> = registry.iter().map(|(_, p)| p.kind.as_str()).collect();

    let dead: Vec<&&str> = CLAIMED
        .iter()
        .filter(|kind| !present.contains(**kind))
        .collect();

    assert!(
        dead.is_empty(),
        "`is_two_pin_symbol_kind` matches {dead:?}, but no part in \
         registry/parts declares that `kind` (registry has: {present:?}). \
         A dead entry reads as coverage while doing nothing — delete it, \
         or fix the `kind` on the part it was meant to cover."
    );

    // The complement direction: a claimed kind that vanished from the
    // implementation while staying in `CLAIMED` is a silent coverage
    // loss for a real part.
    let dropped: Vec<&&str> = CLAIMED
        .iter()
        .filter(|kind| !is_two_pin_symbol_kind(kind))
        .collect();
    assert!(
        dropped.is_empty(),
        "{dropped:?} is still claimed to be a two-pin symbol kind but \
         `is_two_pin_symbol_kind` no longer matches it."
    );
}

#[test]
fn two_pin_parts_of_claimed_kinds_are_classified_two_pin() {
    // The concrete regression: these registry parts are 2-pin and are
    // drawn as a bare two-pin symbol by the exporter, so the placer
    // must reserve the two-pin body. Before the dedup, `inductor` and
    // `switch` fell through to the multi-pin model and reserved
    // >= 15.24 x 10.16 mm for a 7.62 x 4.0 mm symbol.
    let registry = registry();
    let mut unclassified: Vec<(String, String)> = registry
        .iter()
        .filter(|(_, part)| {
            part.pins.len() == 2
                && CLAIMED.contains(&part.kind.as_str())
                && !is_two_pin_symbol_kind(&part.kind)
        })
        .map(|(id, part)| (id.as_str().to_string(), part.kind.clone()))
        .collect();
    unclassified.sort();

    assert!(
        unclassified.is_empty(),
        "2-pin parts of a two-pin kind are not classified as such, so \
         `body_size_for_part` reserves a multi-pin body for them: \
         {unclassified:?}"
    );

    // Spot-check that the corpus actually exercises the two cases the
    // bug report named, so this test cannot quietly stop covering
    // them if the registry changes.
    let two_pin_kinds: BTreeSet<&str> = registry
        .iter()
        .filter(|(_, p)| p.pins.len() == 2 && is_two_pin_symbol_kind(&p.kind))
        .map(|(_, p)| p.kind.as_str())
        .collect();
    for expected in ["inductor", "switch"] {
        assert!(
            two_pin_kinds.contains(expected),
            "no 2-pin `{expected}` part in the registry, so this test no \
             longer covers the divergence it was written for. Found: \
             {two_pin_kinds:?}"
        );
    }
}

/// The bug this file exists for was a *consumer* disagreeing with the
/// canonical list, so assert the consumer's observable output rather
/// than the list. `body_size_for_part` prefers a loaded KiCad symbol's
/// real bbox; clearing `kicad_symbol` forces the synthesized fallback,
/// which is the branch that carried the stale predicate.
#[test]
fn body_sizing_treats_two_pin_inductor_and_switch_as_two_pin() {
    let registry = registry();

    for kind in ["inductor", "switch"] {
        let part = registry
            .iter()
            .filter(|(_, p)| p.kind == kind && p.pins.len() == 2)
            .map(|(_, p)| p.clone())
            .next()
            .unwrap_or_else(|| panic!("no 2-pin `{kind}` part in the registry"));

        let mut stripped = part.clone();
        stripped.kicad_symbol = None;

        let (w, h) = synth_layout::body_size_for_part(&stripped);
        assert!(
            (w - 7.62).abs() < 1e-9 && (h - 4.0).abs() < 1e-9,
            "{} (kind `{kind}`) is drawn and routed as a bare two-pin \
             symbol, so the layout must reserve the two-pin body \
             7.62 x 4.0 mm; got {w} x {h}. The placer and the router \
             disagree about this part.",
            part.id
        );
    }
}
