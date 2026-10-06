// SPDX-License-Identifier: Apache-2.0

use synth_diagnostics::{Diagnostic, DiagnosticBuilder, Location, Severity};
use synth_geometry::{
    calculate_microstrip_z0, derive_microstrip_width_mm, outer_microstrip_params, MicrostripParams,
};
use synth_ir::{Board, DiffPair, Net};

use crate::{ErcCategory, ErcRule};

const ESTIMATE: &str = " (an estimate, about 10 percent; does not replace expert analysis)";
const ERROR_DEVIATION: f64 = 0.20;
const WARNING_DEVIATION: f64 = 0.10;

struct Controlled<'a> {
    net: &'a Net,
    dp: &'a DiffPair,
    target_ohms: f64,
    outer: MicrostripParams,
}

fn controlled(board: &Board) -> impl Iterator<Item = Controlled<'_>> {
    let outer = board.stackup.as_ref().and_then(outer_microstrip_params);
    board
        .single_ended_impedance_nets()
        .into_iter()
        .filter_map(move |(net, dp, impedance)| {
            Some(Controlled {
                net,
                dp,
                target_ohms: impedance.to_ohms(),
                outer: outer.clone()?,
            })
        })
}

fn diagnostic(
    code: &str,
    severity: Severity,
    title: &str,
    file: &str,
    c: &Controlled<'_>,
) -> DiagnosticBuilder {
    DiagnosticBuilder::new(code, severity, format!("{title}{ESTIMATE}"))
        .location(Location::from_span(file.to_string(), c.dp.source_span))
        .explanation_url(format!("synth.docs/diagnostics/{code}"))
}

fn min_trace_width_mm() -> f64 {
    synth_ir::Length(synth_drc::ManufacturerProfile::jlc_standard().min_trace_width_nm).to_mm()
}

pub(crate) struct DeclaredWidthRule;

impl ErcRule for DeclaredWidthRule {
    fn code(&self) -> &'static str {
        "E-SYNTH-IMPEDANCE-001"
    }

    fn category(&self) -> ErcCategory {
        ErcCategory::Rf
    }

    fn check(&self, board: &Board, file: &str) -> Vec<Diagnostic> {
        controlled(board)
            .filter_map(|c| {
                let width = board.declared_trace_width(c.net)?.to_mm();
                derive_microstrip_width_mm(c.target_ohms, &c.outer, min_trace_width_mm()).ok()?;
                let z0 = calculate_microstrip_z0(&MicrostripParams {
                    width_mm: width,
                    ..c.outer.clone()
                })
                .z0_ohms;
                let deviation = (z0 - c.target_ohms).abs() / c.target_ohms;
                let severity = if deviation > ERROR_DEVIATION {
                    Severity::Error
                } else if deviation >= WARNING_DEVIATION {
                    Severity::Warning
                } else {
                    return None;
                };
                Some(
                    diagnostic(
                        self.code(),
                        severity,
                        "declared trace width is off the impedance target",
                        file,
                        &c,
                    )
                    .expected(format!(
                        "a width giving about {:.1} ohm on net `{}`",
                        c.target_ohms, c.net.name
                    ))
                    .found(format!(
                        "{width} mm gives {z0:.1} ohm ({:.0} percent off)",
                        deviation * 100.0
                    ))
                    .build(),
                )
            })
            .collect()
    }
}

pub(crate) struct UnreachableTargetRule;

impl ErcRule for UnreachableTargetRule {
    fn code(&self) -> &'static str {
        "E-SYNTH-IMPEDANCE-002"
    }

    fn category(&self) -> ErcCategory {
        ErcCategory::Rf
    }

    fn check(&self, board: &Board, file: &str) -> Vec<Diagnostic> {
        controlled(board)
            .filter_map(|c| {
                let range =
                    derive_microstrip_width_mm(c.target_ohms, &c.outer, min_trace_width_mm())
                        .err()?;
                Some(
                    diagnostic(
                        self.code(),
                        Severity::Error,
                        "impedance target is outside the range this estimate covers",
                        file,
                        &c,
                    )
                    .expected(format!(
                        "a target inside the range this estimate covers ({:.1} to {:.1} ohm on \
                         this stackup)",
                        range.lowest_ohms, range.highest_ohms
                    ))
                    .found(format!("{:.1} ohm on net `{}`", c.target_ohms, c.net.name))
                    .build(),
                )
            })
            .collect()
    }
}

pub(crate) struct InnerLayerNotVerifiedRule;

impl ErcRule for InnerLayerNotVerifiedRule {
    fn code(&self) -> &'static str {
        "W-SYNTH-IMPEDANCE-001"
    }

    fn category(&self) -> ErcCategory {
        ErcCategory::Rf
    }

    fn check(&self, board: &Board, file: &str) -> Vec<Diagnostic> {
        if board.stackup.as_ref().is_none_or(|s| s.copper_count() <= 2) {
            return Vec::new();
        }
        controlled(board)
            .map(|c| {
                diagnostic(
                    self.code(),
                    Severity::Warning,
                    "impedance not verified on inner layers",
                    file,
                    &c,
                )
                .expected("a controlled-impedance net routed on an outer layer")
                .found(format!(
                    "net `{}` may be routed on an inner copper layer, where impedance is not \
                     verified; only outer-layer microstrip is derived and checked",
                    c.net.name
                ))
                .build()
            })
            .collect()
    }
}
