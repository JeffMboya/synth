// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;

use synth_registry::qualify::{FootprintFacts, PadFacts, PadSide, SymbolFacts};

use crate::kicad_footprint_loader::{self, PadCopperLayers};

#[derive(Debug, Default, Clone, Copy)]
pub struct InstalledKicad;

fn side(layers: PadCopperLayers) -> PadSide {
    match layers {
        PadCopperLayers::Front => PadSide::Front,
        PadCopperLayers::Back => PadSide::Back,
        PadCopperLayers::Both => PadSide::Both,
        PadCopperLayers::None => PadSide::NoCopper,
    }
}

impl FootprintFacts for InstalledKicad {
    fn pads(&self, lib_id: &str) -> Option<Vec<PadFacts>> {
        Some(
            kicad_footprint_loader::pads(lib_id)?
                .into_iter()
                .map(|pad| PadFacts {
                    number: pad.number,
                    center_mm: pad.center_mm,
                    size_mm: pad.size_mm,
                    side: side(pad.copper_layers),
                    is_npth: pad.is_npth,
                })
                .collect(),
        )
    }
}

impl SymbolFacts for InstalledKicad {
    fn pin_numbers(&self, lib_id: &str) -> Option<BTreeSet<String>> {
        synth_registry::kicad_pin_numbers(lib_id)
    }
}
