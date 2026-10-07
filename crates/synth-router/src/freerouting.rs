// SPDX-License-Identifier: Apache-2.0

//! The FreeRouting adapter.
//!
//! FreeRouting is the default routing path: the JAR and, where present,
//! the Java runtime are managed by the project, and the board is routed
//! through a Specctra round trip.
//!
//! Three details carry most of the weight:
//!
//! - **The router reads the baseline, not the delivery path.** Routing
//!   in place would make a failed run destructive.
//! - **`--clean-netlist` is on by default.** Routing a board that already
//!   carries Synth copper lets that copper survive the round trip and be
//!   mistaken for a route the engine produced. Routing from a freshly
//!   written netlist is what makes the result attributable.
//! - **Exit code zero is not success.** The engine exits `0` after routing
//!   nothing at all, so the candidate is checked for actually existing
//!   before it is handed to validation.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::capability::{which, BUNDLED_JRE_DIR, PINNED_FREEROUTING_JAR};
use crate::contract::{FreeroutingOptions, RouteRequest, RouterEngine};
use crate::failure::{RouterFailure, RouterFailureReason, RouterStage};
use crate::result::{RouteArtifacts, RouteStatistics, RouterProvenance};

/// The helper script that drives the JAR, the Specctra export, and the
/// SES import.
pub const HELPER: &str = "freeroute_autoroute.py";

/// What an adapter hands back for validation.
///
/// A candidate is "the board the engine wrote", not "a good board". Every
/// field here is either a path or something the engine claimed; the
/// verdicts are reached later, from the file itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    /// Where the engine wrote its board.
    pub candidate_path: PathBuf,
    /// Paths of everything retained, already filtered to what exists.
    pub artifacts: RouteArtifacts,
    pub provenance: RouterProvenance,
    pub statistics: RouteStatistics,
}

impl Candidate {
    /// Replace the delivered board with this candidate.
    ///
    /// A rename, so the baseline is untouched either way: validation runs
    /// before this is called, and a validated candidate is the only thing
    /// that ever reaches it.
    pub fn install(&self, delivered: &Path) -> Result<(), String> {
        std::fs::rename(&self.candidate_path, delivered).map_err(|e| {
            format!(
                "could not install {} as {}: {e}",
                self.candidate_path.display(),
                delivered.display()
            )
        })
    }
}

/// FreeRouting's JSON summary, as written by the helper.
///
/// Every field is optional because the helper is a thin wrapper and
/// different FreeRouting releases emit different logs. The report is
/// provenance: validation reads the board, not this.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FreeroutingReport {
    #[serde(default)]
    pub router: String,
    #[serde(default)]
    pub jar: Option<String>,
    #[serde(default)]
    pub java_version: Option<String>,
    #[serde(default)]
    pub passes: Option<u32>,
    #[serde(default)]
    pub threads: Option<u32>,
    #[serde(default)]
    pub ses_path: Option<String>,
    #[serde(default)]
    pub imported_segments: Option<usize>,
    #[serde(default)]
    pub imported_vias: Option<usize>,
    #[serde(default)]
    pub notes: Vec<String>,
}

/// Build the command line for a FreeRouting run.
///
/// Pure so the argv can be asserted in a test without a JVM: this is the
/// boundary where a wrong flag silently changes routing behaviour, and it
/// is the cheapest place to pin it.
#[must_use]
pub fn build_command(
    request: &RouteRequest,
    helper: &Path,
    jar: &Path,
    java: &Path,
) -> Vec<String> {
    let mut args = vec![
        helper.display().to_string(),
        request.router_input_path().display().to_string(),
        request.candidate_path().display().to_string(),
        "--jar".to_string(),
        jar.display().to_string(),
        "--java".to_string(),
        java.display().to_string(),
        // Bounded on both axes. An unbounded pass count is how a router
        // run turns into an unbounded CI job.
        "--passes".to_string(),
        request.limits.max_passes.to_string(),
        "--threads".to_string(),
        request.limits.max_threads.to_string(),
        "--ses-output".to_string(),
        request.session_log_path().display().to_string(),
        "--report".to_string(),
        request.router_report_path().display().to_string(),
    ];
    if request.freerouting.clean_netlist {
        args.push("--clean-netlist".to_string());
    }
    if !request.freerouting.retain_session {
        args.push("--no-retain-session".to_string());
    }
    args
}

/// Run FreeRouting over the request's baseline.
///
/// Returns a [`Candidate`] when the engine produced a board, and a
/// structured [`RouterFailure`] when it did not — including when it
/// exited `0` and wrote nothing, which is FreeRouting's signature
/// failure mode.
#[allow(clippy::too_many_lines)]
pub fn route(request: &RouteRequest) -> Result<Candidate, RouterFailure> {
    let prerequisites = Prerequisite::resolve(request)?;
    run_with(request, &prerequisites)
}

/// Everything needed to invoke the engine, resolved up front.
///
/// Resolving first means a missing JAR, runtime, or checkout is reported
/// as a capability error before any work starts, rather than as a spawn
/// failure from inside the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Prerequisite {
    python: PathBuf,
    jar: PathBuf,
    java: PathBuf,
    helper: PathBuf,
}

impl Prerequisite {
    fn resolve(_request: &RouteRequest) -> Result<Self, RouterFailure> {
        let jar = resolve_jar().ok_or_else(|| {
            RouterFailure::for_stage(
                RouterEngine::Freerouting,
                RouterStage::Capability,
                RouterFailureReason::JarMissing,
                format!(
                    "no FreeRouting JAR found; looked for {PINNED_FREEROUTING_JAR} under \
                     tools/freerouting/. Set SYNTH_FREEROUTING_JAR or pass --freerouting-jar."
                ),
            )
        })?;
        let java = resolve_java().ok_or_else(|| {
            RouterFailure::for_stage(
                RouterEngine::Freerouting,
                RouterStage::Capability,
                RouterFailureReason::JavaMissing,
                format!("no Java runtime found; looked at {BUNDLED_JRE_DIR}/bin/java and `java`"),
            )
        })?;
        let python = resolve_python().ok_or_else(|| {
            RouterFailure::for_stage(
                RouterEngine::Freerouting,
                RouterStage::Capability,
                RouterFailureReason::NotInstalled,
                "no `python3` interpreter found to run tools/freeroute_autoroute.py",
            )
        })?;
        let helper = crate::repo_tool(HELPER).ok_or_else(|| {
            RouterFailure::for_stage(
                RouterEngine::Freerouting,
                RouterStage::Capability,
                RouterFailureReason::NotInstalled,
                format!("{HELPER} was not found in the repository's tools/ directory"),
            )
        })?;

        Ok(Self {
            python,
            jar,
            java,
            helper,
        })
    }
}

/// Invoke the helper and classify the outcome.
#[allow(clippy::too_many_lines)]
fn run_with(
    request: &RouteRequest,
    prerequisites: &Prerequisite,
) -> Result<Candidate, RouterFailure> {
    let Prerequisite {
        python,
        jar,
        java,
        helper,
    } = prerequisites.clone();

    // The duplicate-run guard is held by `crate::route`, the single
    // supported entry point. Taking it again here would refuse every run.
    let _ = std::fs::remove_file(request.candidate_path());
    if let Some(parent) = request.candidate_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let argv = build_command(request, &helper, &jar, &java);
    let start = std::time::Instant::now();
    let invocation = crate::process::run(
        &python,
        &argv,
        request.limits.wall_clock,
        request.limits.max_output_bytes,
    )
    .map_err(|error| {
        RouterFailure::for_stage(
            RouterEngine::Freerouting,
            RouterStage::Route,
            if error.not_found {
                RouterFailureReason::NotInstalled
            } else {
                RouterFailureReason::SpawnFailed
            },
            error.detail,
        )
    })?;
    let duration_ms = start.elapsed().as_millis() as u64;

    // Retain whatever the engine said, success or failure: the log is what
    // makes a failed run diagnosable without re-running it.
    let log_path = request.session_log_path();
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&log_path, invocation.combined_output());

    let report = read_report(&request.router_report_path());
    let mut settings = BTreeMap::new();
    settings.insert("passes".to_string(), request.limits.max_passes.to_string());
    settings.insert(
        "threads".to_string(),
        request.limits.max_threads.to_string(),
    );
    settings.insert(
        "clean_netlist".to_string(),
        request.freerouting.clean_netlist.to_string(),
    );
    settings.insert(
        "wall_clock_secs".to_string(),
        request.limits.wall_clock.as_secs().to_string(),
    );
    if invocation.output_truncated {
        settings.insert("output_truncated".to_string(), "true".to_string());
    }

    let provenance = RouterProvenance {
        engine: RouterEngine::Freerouting.as_str().to_string(),
        engine_version: report
            .as_ref()
            .and_then(|r| r.jar.clone())
            .or_else(|| jar.file_name().map(|n| n.to_string_lossy().into_owned())),
        runtime_version: report
            .as_ref()
            .and_then(|r| r.java_version.clone())
            .or_else(|| crate::capability::probe_runtime_version(&java)),
        profile: request.profile_name.clone(),
        command: invocation.command.clone(),
        settings,
        input_hash: request.input_hash.clone(),
        output_hash: String::new(),
        source_revision: request.source_revision.clone(),
        duration_ms,
    };

    check_process_outcome(RouterEngine::Freerouting, HELPER, &invocation, request)?;

    // Exit code zero with no board is FreeRouting's characteristic
    // failure: it completes cleanly after routing nothing.
    if !request.candidate_path().is_file() {
        return Err(RouterFailure::for_stage(
            RouterEngine::Freerouting,
            RouterStage::Import,
            RouterFailureReason::NoOutput,
            format!(
                "FreeRouting exited successfully but wrote no board at {}; the baseline \
                 export is preserved unchanged",
                request.candidate_path().display()
            ),
        )
        .with_exit_code(Some(0)));
    }

    let board = std::fs::read_to_string(request.candidate_path())
        .ok()
        .and_then(|text| crate::pcb_read::PcbBoard::parse(&text).ok());

    Ok(Candidate {
        candidate_path: request.candidate_path(),
        artifacts: RouteArtifacts {
            baseline: request.baseline_path(),
            candidate: Some(request.candidate_path()),
            delivered: Some(request.board_path.clone()),
            session_log: Some(log_path),
            router_report: Some(request.router_report_path()),
            ..RouteArtifacts::default()
        }
        .resolved(),
        provenance,
        statistics: statistics_from(board.as_ref(), report.as_ref()),
    })
}

/// The first non-empty line of a process's output, for a one-line
/// diagnostic that still quotes the engine rather than paraphrasing it.
#[must_use]
pub fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("no output")
        .to_string()
}

/// Turn a bounded invocation into a failure, or `Ok(())` when the process
/// ran to completion.
///
/// Shared by both adapters because the three ways a run can stop early —
/// budget exhausted, cancelled, crashed — mean the same thing whichever
/// engine was running, and reporting them differently would let a reader
/// infer that one engine is better behaved than the other.
///
/// # Errors
/// Returns a [`RouterFailure`] whose stage and reason identify why the
/// process did not finish.
pub fn check_process_outcome(
    engine: RouterEngine,
    helper: &str,
    invocation: &crate::process::Invocation,
    request: &RouteRequest,
) -> Result<(), RouterFailure> {
    if invocation.timed_out {
        return Err(RouterFailure::for_stage(
            engine,
            RouterStage::Route,
            RouterFailureReason::Timeout,
            format!(
                "{engine} did not finish within {}s and was terminated; the baseline \
                 export at {} is preserved",
                request.limits.wall_clock.as_secs(),
                request.baseline_path().display()
            ),
        ));
    }
    if invocation.cancelled {
        return Err(RouterFailure::for_stage(
            engine,
            RouterStage::Route,
            RouterFailureReason::Cancelled,
            format!("the {engine} run was cancelled before it finished"),
        ));
    }
    if invocation.exit_code != Some(0) {
        return Err(RouterFailure::for_stage(
            engine,
            RouterStage::Route,
            RouterFailureReason::Crashed,
            format!(
                "tools/{helper} {}: {}",
                invocation
                    .exit_code
                    .map_or_else(|| "was terminated".to_string(), |c| format!("exited {c}")),
                first_line(&invocation.combined_output())
            ),
        )
        .with_exit_code(invocation.exit_code));
    }
    Ok(())
}

/// Candidate JAR locations, most specific first.
///
/// An explicit configuration always wins; the pinned JAR name is searched
/// before any looser match so a stray newer JAR in the directory cannot
/// silently change which engine version runs.
#[must_use]
pub fn resolve_jar() -> Option<PathBuf> {
    if let Some(configured) = std::env::var_os("SYNTH_FREEROUTING_JAR")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
    {
        return configured.is_file().then_some(configured);
    }
    for base in tool_roots() {
        let jar = base.join("freerouting").join(PINNED_FREEROUTING_JAR);
        if jar.is_file() {
            return Some(jar);
        }
    }
    None
}

fn resolve_java() -> Option<PathBuf> {
    if let Some(configured) = std::env::var_os("SYNTH_FREEROUTING_JAVA")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
    {
        return configured.is_file().then_some(configured);
    }
    for base in tool_roots() {
        let bundled = base.join("jre25").join("bin").join("java");
        if bundled.is_file() {
            return Some(bundled);
        }
    }
    which("java")
}

/// Directories that may hold `tools/freerouting/` and the bundled JRE.
#[must_use]
pub fn tool_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(configured) = std::env::var("SYNTH_TOOLS_DIR") {
        let path = PathBuf::from(configured);
        if !path.as_os_str().is_empty() {
            roots.push(path);
        }
    }
    if let Some(root) = crate::repo_root() {
        roots.push(root.join("tools"));
        if let Some(parent) = root.parent() {
            roots.push(parent.join("tools"));
        }
    }
    roots
}

fn resolve_python() -> Option<PathBuf> {
    which("python3").or_else(|| which("python"))
}

fn read_report(path: &Path) -> Option<FreeroutingReport> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Statistics for the candidate.
///
/// Copper counts come from the board that was actually written, not from
/// the engine's log, so the two can be compared. The engine's own claims
/// are preserved under `engine_findings` rather than being trusted.
#[must_use]
pub fn statistics_from(
    board: Option<&crate::pcb_read::PcbBoard>,
    report: Option<&FreeroutingReport>,
) -> RouteStatistics {
    let mut statistics = RouteStatistics::default();
    let mut findings = BTreeMap::new();
    if let Some(report) = report {
        findings.insert(
            "claimed_imported_segments".to_string(),
            report
                .imported_segments
                .map_or_else(|| "unknown".to_string(), |n| n.to_string()),
        );
        findings.insert(
            "claimed_imported_vias".to_string(),
            report
                .imported_vias
                .map_or_else(|| "unknown".to_string(), |n| n.to_string()),
        );
        for note in &report.notes {
            findings.insert(format!("note.{note}"), String::new());
        }
    }
    if let Some(board) = board {
        statistics.segments = board.segments.len();
        statistics.vias = board.vias.len();
        statistics.wire_length_mm = board.wire_length_mm();
    }
    statistics.engine_findings = findings;
    statistics
}

/// Default options, re-exported so callers do not have to name the module.
#[must_use]
pub fn default_options() -> FreeroutingOptions {
    FreeroutingOptions::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{RouterEngine, RouterLimits};

    fn request() -> RouteRequest {
        let mut request = RouteRequest::new(
            Path::new("/tmp/out/demo.kicad_pcb"),
            Path::new("/tmp/out"),
            RouterEngine::Freerouting,
        );
        request.stem = "demo".to_string();
        request.limits = RouterLimits::default();
        request
    }

    fn argv(request: &RouteRequest) -> String {
        build_command(
            request,
            Path::new("/repo/tools/freeroute_autoroute.py"),
            Path::new("/repo/tools/freerouting/freerouting-2.4.1.jar"),
            Path::new("/opt/java/bin/java"),
        )
        .join(" ")
    }

    #[test]
    fn the_command_pins_the_limits_rather_than_leaving_them_unbounded() {
        let joined = argv(&request());
        assert!(joined.contains("--passes 40"), "{joined}");
        assert!(joined.contains("--threads 4"), "{joined}");
        assert!(joined.contains("--jar"), "{joined}");
        assert!(joined.contains("--java"), "{joined}");
        // Routing from a clean netlist is the difference between a real
        // route and stale copper that looks like one.
        assert!(joined.contains("--clean-netlist"), "{joined}");
        // The retained session and report are what make a failed run
        // diagnosable.
        assert!(joined.contains("--ses-output"), "{joined}");
        assert!(joined.contains("--report"), "{joined}");
    }

    #[test]
    fn the_router_is_handed_the_baseline_never_the_delivery_path() {
        let args = build_command(
            &request(),
            Path::new("/repo/tools/freeroute_autoroute.py"),
            Path::new("jar"),
            Path::new("java"),
        );
        assert!(args[1].ends_with("demo.synth.kicad_pcb"), "{args:?}");
        assert!(args[2].ends_with("demo.freerouting.kicad_pcb"), "{args:?}");
    }

    #[test]
    fn a_reduced_pass_count_reaches_the_command() {
        let request = RouteRequest {
            limits: RouterLimits::default().with_passes(3).with_threads(1),
            ..request()
        };
        let joined = argv(&request);
        assert!(joined.contains("--passes 3"), "{joined}");
        assert!(joined.contains("--threads 1"), "{joined}");
    }

    #[test]
    fn cleaning_the_netlist_can_be_turned_off_and_says_so() {
        // Kept configurable because it is genuinely useful when re-routing
        // a board with trusted existing copper, but the default is the one
        // that makes results attributable.
        let request = RouteRequest {
            freerouting: FreeroutingOptions {
                clean_netlist: false,
                retain_session: false,
            },
            ..request()
        };
        let joined = argv(&request);
        assert!(!joined.contains("--clean-netlist"), "{joined}");
        assert!(joined.contains("--no-retain-session"), "{joined}");
    }

    #[test]
    fn statistics_come_from_the_board_that_was_written() {
        let board =
            crate::pcb_read::PcbBoard::parse(crate::pcb_read::SAMPLE_BOARD).expect("parses");
        let report = FreeroutingReport {
            imported_segments: Some(999),
            imported_vias: Some(111),
            ..FreeroutingReport::default()
        };
        let statistics = statistics_from(Some(&board), Some(&report));
        assert_eq!(statistics.segments, 3);
        assert_eq!(statistics.vias, 1);
        // The engine's claim is recorded but is not what gets counted.
        assert_eq!(
            statistics
                .engine_findings
                .get("claimed_imported_segments")
                .map(String::as_str),
            Some("999")
        );
        assert!(statistics.wire_length_mm > 0.0);
    }

    #[test]
    fn a_missing_report_leaves_statistics_from_the_board_alone() {
        let board =
            crate::pcb_read::PcbBoard::parse(crate::pcb_read::SAMPLE_BOARD).expect("parses");
        let statistics = statistics_from(Some(&board), None);
        assert_eq!(statistics.segments, 3);
        assert!(statistics.engine_findings.is_empty());
    }

    #[test]
    fn an_unreadable_board_yields_zero_copper_rather_than_a_guess() {
        let statistics = statistics_from(None, None);
        assert_eq!(statistics.segments, 0);
        assert_eq!(statistics.vias, 0);
        assert_eq!(statistics.wire_length_mm, 0.0);
    }

    #[test]
    fn installing_replaces_the_delivered_board_and_leaves_the_baseline() {
        let dir = std::env::temp_dir().join(format!("synth_fr_install_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch");
        let baseline = dir.join("demo.synth.kicad_pcb");
        let candidate = dir.join("demo.freerouting.kicad_pcb");
        let delivered = dir.join("demo.kicad_pcb");
        std::fs::write(&baseline, b"(kicad_pcb baseline)").expect("baseline");
        std::fs::write(&candidate, b"(kicad_pcb routed)").expect("candidate");

        let subject = Candidate {
            candidate_path: candidate,
            artifacts: RouteArtifacts::default(),
            provenance: RouterProvenance::default(),
            statistics: RouteStatistics::default(),
        };
        subject.install(&delivered).expect("install");

        assert_eq!(
            std::fs::read_to_string(&baseline).unwrap(),
            "(kicad_pcb baseline)"
        );
        assert_eq!(
            std::fs::read_to_string(&delivered).unwrap(),
            "(kicad_pcb routed)"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
