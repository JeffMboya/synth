// SPDX-License-Identifier: Apache-2.0

//! Consistency checks for a declared `stackup { … }`. A board without one
//! is never checked: there is nothing to disagree with.

use synth_diagnostics::{Diagnostic, DiagnosticBuilder, Location, Severity};
use synth_ir::{Board, StackupLayer};

use crate::{ErcCategory, ErcRule};

fn error(code: &str, title: &str, file: &str, span: synth_diagnostics::Span) -> DiagnosticBuilder {
    DiagnosticBuilder::new(code, Severity::Error, title)
        .location(Location::from_span(file.to_string(), span))
        .explanation_url(format!("synth.docs/diagnostics/{code}"))
}

// -----------------------------------------------------------------------------
// E-SYNTH-STACKUP-001 — copper count disagrees with `layers`, or `layers`
// is one the exporter cannot write a stackup for
// -----------------------------------------------------------------------------

pub(crate) struct StackupLayerCountRule;

impl ErcRule for StackupLayerCountRule {
    fn code(&self) -> &'static str {
        "E-SYNTH-STACKUP-001"
    }

    fn category(&self) -> ErcCategory {
        ErcCategory::Board
    }

    fn check(&self, board: &Board, file: &str) -> Vec<Diagnostic> {
        let Some(stackup) = board.stackup.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let copper = stackup.copper_count();
        if copper != board.layers as usize {
            out.push(
                error(
                    self.code(),
                    "stackup copper layer count does not match `layers`",
                    file,
                    stackup.source_span,
                )
                .expected(format!(
                    "{} copper layers, as declared by `layers`",
                    board.layers
                ))
                .found(format!("{copper} copper layers in the stackup"))
                .build(),
            );
        }
        if !matches!(board.layers, 2 | 4 | 6) {
            out.push(
                error(
                    self.code(),
                    "a stackup needs a board with 2, 4 or 6 layers",
                    file,
                    stackup.source_span,
                )
                .expected("`layers` of 2, 4 or 6, the layer counts the exporter writes")
                .found(format!("layers {}", board.layers))
                .build(),
            );
        }
        out
    }
}

// -----------------------------------------------------------------------------
// E-SYNTH-STACKUP-002 — copper and insulator do not alternate
// -----------------------------------------------------------------------------

pub(crate) struct StackupAlternationRule;

impl ErcRule for StackupAlternationRule {
    fn code(&self) -> &'static str {
        "E-SYNTH-STACKUP-002"
    }

    fn category(&self) -> ErcCategory {
        ErcCategory::Board
    }

    fn check(&self, board: &Board, file: &str) -> Vec<Diagnostic> {
        let Some(stackup) = board.stackup.as_ref() else {
            return Vec::new();
        };
        let misplaced = stackup
            .layers
            .iter()
            .enumerate()
            .find(|(i, layer)| layer.is_copper() != (i % 2 == 0));
        let (span, found) = match (misplaced, stackup.layers.last()) {
            (Some((i, layer)), _) => (
                layer_span(layer),
                format!("{} at position {}", kind_name(layer), i + 1),
            ),
            (None, Some(last)) if !last.is_copper() => (
                layer_span(last),
                format!("the stackup ends with an {}", kind_name(last)),
            ),
            _ => return Vec::new(),
        };
        vec![error(
            self.code(),
            "stackup copper and insulator layers do not alternate",
            file,
            span,
        )
        .expected("copper first, then alternating insulator and copper, ending with copper")
        .found(found)
        .build()]
    }
}

// -----------------------------------------------------------------------------
// E-SYNTH-STACKUP-003 — non-positive thickness or dielectric constant below 1
// -----------------------------------------------------------------------------

pub(crate) struct StackupNonPositiveRule;

impl ErcRule for StackupNonPositiveRule {
    fn code(&self) -> &'static str {
        "E-SYNTH-STACKUP-003"
    }

    fn category(&self) -> ErcCategory {
        ErcCategory::Board
    }

    fn check(&self, board: &Board, file: &str) -> Vec<Diagnostic> {
        let Some(stackup) = board.stackup.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for layer in &stackup.layers {
            if layer.thickness().0 <= 0 {
                out.push(
                    error(
                        self.code(),
                        "stackup layer thickness is not positive",
                        file,
                        layer_span(layer),
                    )
                    .expected("a thickness greater than zero")
                    .found(format!("{} mm", layer.thickness().to_mm()))
                    .build(),
                );
            }
            if let StackupLayer::Insulator { er, .. } = layer {
                if er.0 < MIN_ER_MICRO {
                    out.push(
                        error(
                            self.code(),
                            "stackup dielectric constant is below 1",
                            file,
                            layer_span(layer),
                        )
                        .expected("`er` of at least 1, the permittivity of vacuum")
                        .found(format!("er {}", er.to_f64()))
                        .build(),
                    );
                }
            }
        }
        out
    }
}

const MIN_ER_MICRO: i64 = 1_000_000;

fn layer_span(layer: &StackupLayer) -> synth_diagnostics::Span {
    match layer {
        StackupLayer::Copper { source_span, .. } | StackupLayer::Insulator { source_span, .. } => {
            *source_span
        }
    }
}

fn kind_name(layer: &StackupLayer) -> &'static str {
    if layer.is_copper() {
        "copper"
    } else {
        "insulator"
    }
}
