// SPDX-License-Identifier: Apache-2.0

//! Evidence contract for native (out-of-process) verification stages.
//!
//! Synth's own ERC, DRC, placement and routing are in-process and
//! deterministic: they either pass or fail. The stages that shell out to
//! `kicad-cli` have a third outcome — the check *could not be performed*.
//! A missing binary, a missing symbol/footprint library, a crash, a
//! timeout, an unparseable report, or an unsupported KiCad version all
//! leave us without evidence either way.
//!
//! Collapsing that third outcome into "pass" is how a production-looking
//! package escapes without native verification, so it gets its own
//! variant here ([`NativeCheckStatus::Unknown`]) and its own blocking
//! semantics ([`NativeCheckStatus::is_trusted`]).
//!
//! The type is deliberately data-only. Process spawning lives with the
//! stage that owns the tool (`synth_drc::kicad_cli`); this crate owns the
//! wire format agents and CI consume.

use serde::{Deserialize, Serialize};

/// Outcome of a verification stage that runs an external tool.
///
/// `Unknown` is not a soft pass. Every release-gating consumer must treat
/// it as blocking — see [`NativeCheckStatus::is_trusted`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeCheckStatus {
    /// The tool ran and reported no blocking violations.
    Pass,
    /// The tool ran and reported at least one blocking violation.
    Fail,
    /// The tool did not produce trustworthy evidence. The design is
    /// neither proven good nor proven bad.
    Unknown,
}

impl NativeCheckStatus {
    /// Whether this status is evidence the design was actually checked.
    ///
    /// Only [`Self::Pass`] is. `Fail` and `Unknown` both block a release
    /// gate, for different reasons: one found problems, the other found
    /// nothing at all.
    pub fn is_trusted(self) -> bool {
        matches!(self, Self::Pass)
    }

    /// Whether this status must block a production/fabrication result.
    pub fn is_blocking(self) -> bool {
        !self.is_trusted()
    }

    /// Stable lowercase wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for NativeCheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why a native check produced [`NativeCheckStatus::Unknown`].
///
/// Machine-stable: agents and CI branch on these, so variants are added
/// but never renamed or repurposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownReason {
    /// The executable was not found on `PATH` (or at `KICAD_CLI`).
    NotInstalled,
    /// The executable could not be spawned for some other reason.
    SpawnFailed,
    /// The tool exceeded its wall-clock budget and was killed.
    Timeout,
    /// The tool exited non-zero. Covers crashes and missing libraries,
    /// which `kicad-cli` surfaces as a failure code plus stderr.
    CommandFailed,
    /// The tool exited cleanly but wrote no report file.
    ReportMissing,
    /// The report file exists but could not be read.
    ReportUnreadable,
    /// The report file is not valid JSON.
    ReportMalformed,
    /// The report parsed as JSON but has none of the keys this schema
    /// version expects. A silently-empty result from an unrecognized
    /// layout is the worst kind of false clean, so it is never treated
    /// as "zero violations".
    ReportUnrecognized,
    /// The tool's version is outside the range Synth knows how to read.
    UnsupportedVersion,
}

impl UnknownReason {
    /// Stable snake_case wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotInstalled => "not_installed",
            Self::SpawnFailed => "spawn_failed",
            Self::Timeout => "timeout",
            Self::CommandFailed => "command_failed",
            Self::ReportMissing => "report_missing",
            Self::ReportUnreadable => "report_unreadable",
            Self::ReportMalformed => "report_malformed",
            Self::ReportUnrecognized => "report_unrecognized",
            Self::UnsupportedVersion => "unsupported_version",
        }
    }

    /// One-line operator-facing explanation, including the usual remedy.
    pub fn explain(self) -> &'static str {
        match self {
            Self::NotInstalled => {
                "kicad-cli was not found; install KiCad or set KICAD_CLI to its path"
            }
            Self::SpawnFailed => "the verification tool could not be started",
            Self::Timeout => "the verification tool exceeded its time budget and was terminated",
            Self::CommandFailed => {
                "the verification tool exited with a failure code (see stderr; a missing \
                 symbol/footprint library is the common cause)"
            }
            Self::ReportMissing => "the verification tool wrote no report file",
            Self::ReportUnreadable => "the report file could not be read",
            Self::ReportMalformed => "the report file is not valid JSON",
            Self::ReportUnrecognized => {
                "the report JSON has an unrecognized layout, so an empty result cannot be \
                 read as a clean run"
            }
            Self::UnsupportedVersion => "the tool version is outside the supported range",
        }
    }
}

impl std::fmt::Display for UnknownReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What a native verification stage actually did, in enough detail to
/// audit the result without re-running it.
///
/// `#[serde(skip_serializing_if)]` keeps a clean pass compact while an
/// `unknown` carries the full forensic trail issue #42 requires: tool,
/// version, exact argv, stderr, and reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeCheckEvidence {
    /// Logical stage name, e.g. `kicad_erc` or `kicad_drc`.
    pub stage: String,
    /// The tool invoked, as resolved (honours `KICAD_CLI`).
    pub tool: String,
    /// Tool version string, when it could be queried.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,
    /// The exact argv, so a reviewer can reproduce the run verbatim.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    pub status: NativeCheckStatus,
    /// Present exactly when `status` is `unknown`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<UnknownReason>,
    /// Human-readable detail for `reason` (exit code, io error, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Captured stderr, truncated to [`STDERR_CAPTURE_LIMIT`] bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stderr: Option<String>,
    /// Blocking violations the tool reported. Meaningful only when
    /// `status` is `pass` or `fail`.
    #[serde(default)]
    pub violations: usize,
}

/// Upper bound on captured stderr, so one chatty tool run cannot bloat a
/// release manifest. Chosen to comfortably hold a KiCad library-load
/// failure, which is the stderr an operator most needs to read.
pub const STDERR_CAPTURE_LIMIT: usize = 8 * 1024;

impl NativeCheckEvidence {
    /// Evidence for a stage that ran and produced a verdict.
    ///
    /// `violations == 0` is a pass; anything else is a fail.
    pub fn concluded(
        stage: impl Into<String>,
        tool: impl Into<String>,
        command: Vec<String>,
        violations: usize,
    ) -> Self {
        Self {
            stage: stage.into(),
            tool: tool.into(),
            tool_version: None,
            command,
            status: if violations == 0 {
                NativeCheckStatus::Pass
            } else {
                NativeCheckStatus::Fail
            },
            reason: None,
            detail: None,
            stderr: None,
            violations,
        }
    }

    /// Evidence for a stage that could not be performed.
    pub fn unknown(
        stage: impl Into<String>,
        tool: impl Into<String>,
        command: Vec<String>,
        reason: UnknownReason,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            stage: stage.into(),
            tool: tool.into(),
            tool_version: None,
            command,
            status: NativeCheckStatus::Unknown,
            reason: Some(reason),
            detail: Some(detail.into()),
            stderr: None,
            violations: 0,
        }
    }

    /// Attach the tool version, when known.
    pub fn with_version(mut self, version: Option<String>) -> Self {
        self.tool_version = version;
        self
    }

    /// Attach stderr, truncated to [`STDERR_CAPTURE_LIMIT`]. Empty
    /// stderr is dropped rather than serialized as `""`.
    pub fn with_stderr(mut self, stderr: &str) -> Self {
        let trimmed = stderr.trim_end();
        if trimmed.is_empty() {
            self.stderr = None;
            return self;
        }
        self.stderr = Some(truncate_on_char_boundary(trimmed, STDERR_CAPTURE_LIMIT));
        self
    }

    /// Whether this stage is evidence the design was actually checked.
    pub fn is_trusted(&self) -> bool {
        self.status.is_trusted()
    }

    /// One line for human output, e.g.
    /// `kicad_drc: unknown (not_installed) — kicad-cli was not found; ...`
    pub fn summary_line(&self) -> String {
        match (self.status, self.reason) {
            (NativeCheckStatus::Unknown, Some(reason)) => {
                format!(
                    "{}: unknown ({}) — {}",
                    self.stage,
                    reason.as_str(),
                    reason.explain()
                )
            }
            (NativeCheckStatus::Unknown, None) => format!("{}: unknown", self.stage),
            (status, _) => format!(
                "{}: {} ({} violation{})",
                self.stage,
                status.as_str(),
                self.violations,
                if self.violations == 1 { "" } else { "s" }
            ),
        }
    }
}

/// Truncate to at most `limit` bytes without splitting a UTF-8
/// character, appending an elision marker when anything was dropped.
fn truncate_on_char_boundary(s: &str, limit: usize) -> String {
    if s.len() <= limit {
        return s.to_string();
    }
    let mut end = limit;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… [truncated]", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_is_not_trusted_and_blocks() {
        let e = NativeCheckEvidence::unknown(
            "kicad_drc",
            "kicad-cli",
            vec!["kicad-cli".into(), "pcb".into(), "drc".into()],
            UnknownReason::NotInstalled,
            "No such file or directory",
        );
        assert!(!e.is_trusted());
        assert!(e.status.is_blocking());
        assert_eq!(e.status, NativeCheckStatus::Unknown);
    }

    #[test]
    fn clean_run_passes_and_violations_fail() {
        let clean = NativeCheckEvidence::concluded("kicad_erc", "kicad-cli", vec![], 0);
        assert!(clean.is_trusted());
        let dirty = NativeCheckEvidence::concluded("kicad_erc", "kicad-cli", vec![], 3);
        assert_eq!(dirty.status, NativeCheckStatus::Fail);
        assert!(dirty.status.is_blocking());
    }

    #[test]
    fn status_and_reason_wire_strings_are_stable() {
        assert_eq!(
            serde_json::to_string(&NativeCheckStatus::Unknown).unwrap(),
            "\"unknown\""
        );
        assert_eq!(
            serde_json::to_string(&UnknownReason::ReportUnrecognized).unwrap(),
            "\"report_unrecognized\""
        );
        assert_eq!(UnknownReason::Timeout.as_str(), "timeout");
    }

    #[test]
    fn pass_evidence_omits_unknown_only_fields() {
        let json = serde_json::to_value(NativeCheckEvidence::concluded(
            "kicad_drc",
            "kicad-cli",
            vec!["kicad-cli".into()],
            0,
        ))
        .unwrap();
        assert_eq!(json["status"], "pass");
        assert!(json.get("reason").is_none());
        assert!(json.get("detail").is_none());
        assert!(json.get("stderr").is_none());
    }

    #[test]
    fn stderr_is_truncated_on_a_char_boundary() {
        let noisy = "é".repeat(STDERR_CAPTURE_LIMIT);
        let e =
            NativeCheckEvidence::concluded("kicad_drc", "kicad-cli", vec![], 0).with_stderr(&noisy);
        let captured = e.stderr.expect("stderr retained");
        assert!(captured.ends_with("… [truncated]"));
        // Truncation must not have produced invalid UTF-8 or split a char.
        assert!(captured.starts_with('é'));
    }

    #[test]
    fn empty_stderr_is_dropped_rather_than_serialized() {
        let e =
            NativeCheckEvidence::concluded("kicad_drc", "kicad-cli", vec![], 0).with_stderr("  \n");
        assert!(e.stderr.is_none());
    }

    #[test]
    fn summary_line_names_the_reason_for_unknown() {
        let e = NativeCheckEvidence::unknown(
            "kicad_erc",
            "kicad-cli",
            vec![],
            UnknownReason::Timeout,
            "exceeded 60s",
        );
        let line = e.summary_line();
        assert!(line.contains("unknown"), "{line}");
        assert!(line.contains("timeout"), "{line}");
    }
}
