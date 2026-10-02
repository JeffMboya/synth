// SPDX-License-Identifier: Apache-2.0

//! One place where Synth invokes `kicad-cli`, with a timeout and a
//! classified outcome.
//!
//! Two stages shell out to KiCad for verification evidence: schematic ERC
//! (`synth_kicad::run_kicad_erc`) and native PCB DRC
//! ([`crate::run_kicad_cli_drc`]). Both need identical handling of the
//! same six failure modes, and getting any one of them wrong turns a
//! release gate into a rubber stamp. So the spawn, the wall-clock budget,
//! the stderr capture, and the version query live here rather than being
//! reimplemented per stage.
//!
//! This module sits in `synth-drc` because `synth-kicad` already depends
//! on it, so both callers can reach it without a new edge in the crate
//! graph. The wire types it reports into are in `synth-diagnostics`.
//!
//! # Why reader threads
//!
//! A poll-`try_wait` loop over a child with piped stdio deadlocks as soon
//! as the child writes more than one pipe buffer: the child blocks on
//! write, so it never exits, so the loop never sees it exit. `kicad-cli`
//! is normally quiet, but a library-load failure is exactly the case we
//! most need to capture and exactly the case that gets chatty. Each pipe
//! therefore gets a draining thread.
//!
//! # Why the drain is time-bounded
//!
//! Killing a timed-out child does not necessarily close its pipes. Most
//! shells (dash, as `/bin/sh` on Debian) fork rather than exec, so a
//! `kicad-cli` wrapper script that hangs leaves an orphaned grandchild
//! holding the inherited write ends. Joining the readers unconditionally
//! then blocks until that orphan exits on its own — the timeout bounds
//! nothing at all, which is the bug this module exists to prevent. The
//! readers therefore report through channels and are abandoned after
//! [`PIPE_DRAIN_GRACE`], leaving output best-effort rather than letting a
//! hung tool extend our wall clock without limit.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use synth_diagnostics::UnknownReason;

/// Lowest `kicad-cli` major version whose JSON report schema Synth can
/// read. KiCad 7 and earlier predate the JSON ERC/DRC reports entirely,
/// so their output cannot be interpreted at all.
///
/// There is deliberately no upper bound. Pinning a maximum would turn
/// every new KiCad release into a false `unsupported_version`, and the
/// failure that actually matters — a report schema we do not recognize —
/// is caught by the report-shape check instead, which needs no version
/// table to stay correct.
pub const MIN_SUPPORTED_MAJOR: u32 = 8;

/// Default wall-clock budget for one `kicad-cli` verification run.
/// Override with `SYNTH_KICAD_CLI_TIMEOUT_SECS`.
pub const DEFAULT_TIMEOUT_SECS: u64 = 300;

/// Budget for the cheap `kicad-cli version` probe. It either answers
/// immediately or something is badly wrong.
const VERSION_PROBE_TIMEOUT_SECS: u64 = 30;

/// How long to keep waiting for a pipe after the child has gone.
///
/// On a clean exit the data is already buffered and arrives at once; this
/// only caps the pathological case where an orphaned grandchild still
/// holds the write end. Long enough not to truncate a real report, short
/// enough that it cannot meaningfully extend a run.
pub const PIPE_DRAIN_GRACE: Duration = Duration::from_secs(5);

/// Resolve the `kicad-cli` binary. `KICAD_CLI` lets CI pin a specific
/// build (and lets the tests substitute a stub); otherwise we trust
/// `PATH`. Matches the convention already used by `synth_kicad::fab`.
pub fn binary() -> String {
    std::env::var("KICAD_CLI").unwrap_or_else(|_| "kicad-cli".to_string())
}

/// Wall-clock budget for a verification run, honouring
/// `SYNTH_KICAD_CLI_TIMEOUT_SECS`. A zero or unparseable value falls back
/// to [`DEFAULT_TIMEOUT_SECS`] — a zero timeout would report every run as
/// `unknown`, which is a footgun, not a configuration.
pub fn timeout() -> Duration {
    let secs = std::env::var("SYNTH_KICAD_CLI_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// A completed (or killed) `kicad-cli` invocation.
#[derive(Debug, Clone)]
pub struct Invocation {
    /// The exact argv, binary included, for reproducing the run.
    pub command: Vec<String>,
    pub stdout: Vec<u8>,
    pub stderr: String,
    /// `None` when the process was killed by a signal or by our timeout.
    pub exit_code: Option<i32>,
    pub timed_out: bool,
}

impl Invocation {
    /// Whether the tool exited zero and was not killed.
    pub fn succeeded(&self) -> bool {
        !self.timed_out && self.exit_code == Some(0)
    }

    /// The [`UnknownReason`] for a run that did not exit cleanly, or
    /// `None` when it did.
    pub fn failure_reason(&self) -> Option<UnknownReason> {
        if self.timed_out {
            Some(UnknownReason::Timeout)
        } else if self.exit_code == Some(0) {
            None
        } else {
            Some(UnknownReason::CommandFailed)
        }
    }

    /// Operator-facing detail for [`Self::failure_reason`].
    pub fn failure_detail(&self, budget: Duration) -> String {
        if self.timed_out {
            format!(
                "`{}` did not finish within {}s and was terminated",
                self.command.join(" "),
                budget.as_secs()
            )
        } else {
            match self.exit_code {
                Some(code) => format!("`{}` exited with code {code}", self.command.join(" ")),
                None => format!("`{}` was terminated by a signal", self.command.join(" ")),
            }
        }
    }
}

/// Why a `kicad-cli` process could not be started at all.
#[derive(Debug)]
pub struct SpawnFailure {
    pub command: Vec<String>,
    pub reason: UnknownReason,
    pub detail: String,
}

/// Spawn a thread that reads a pipe to EOF and reports the bytes back.
///
/// The thread is detached rather than joined: if the write end outlives
/// the child (see the module note on orphans) the caller must be able to
/// walk away from it.
fn drain(pipe: Option<impl Read + Send + 'static>) -> Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        // A send failure means the caller already gave up; nothing to do.
        let _ = tx.send(buf);
    });
    rx
}

/// Collect a drained pipe, giving up at `deadline`.
///
/// Returns whatever is available; an abandoned reader yields empty output
/// rather than blocking. Losing stderr is the right trade against hanging:
/// the runs where stderr actually matters are the ones that exited.
///
/// The deadline is shared across both pipes by the caller. Giving each its
/// own [`PIPE_DRAIN_GRACE`] would let a hung tool cost twice the grace.
fn collect(rx: &Receiver<Vec<u8>>, deadline: Instant) -> Vec<u8> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    // Both error arms mean the same thing here: no output to report.
    // Timeout is the abandoned-reader case, Disconnected a panicked one.
    rx.recv_timeout(remaining).unwrap_or_default()
}

/// Run `kicad-cli` with `args` under a wall-clock budget.
///
/// Returns `Err` only when the process could not be started; a tool that
/// ran and failed is an `Ok` [`Invocation`] whose
/// [`Invocation::succeeded`] is false. That split keeps "we have no
/// evidence" and "the tool gave us a verdict" from collapsing into one
/// error path, which is how the old code ended up warning and continuing.
pub fn run(args: &[String], budget: Duration) -> Result<Invocation, SpawnFailure> {
    run_binary(&binary(), args, budget)
}

/// [`run`], but against an explicit executable.
///
/// Exists so the spawn, timeout and pipe-draining behaviour can be
/// tested against stub programs. `KICAD_CLI` is process-global, so
/// setting it per-test would race with every other test in the binary.
pub fn run_binary(
    bin: &str,
    args: &[String],
    budget: Duration,
) -> Result<Invocation, SpawnFailure> {
    let mut command = Vec::with_capacity(args.len() + 1);
    command.push(bin.to_string());
    command.extend(args.iter().cloned());

    let mut child = match Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(source) => {
            let reason = if source.kind() == std::io::ErrorKind::NotFound {
                UnknownReason::NotInstalled
            } else {
                UnknownReason::SpawnFailed
            };
            return Err(SpawnFailure {
                detail: format!("could not start `{bin}`: {source}"),
                command,
                reason,
            });
        }
    };

    // Drain both pipes concurrently; see the module notes on deadlock and
    // on orphaned grandchildren.
    let stdout_reader = drain(child.stdout.take());
    let stderr_reader = drain(child.stderr.take());

    let deadline = Instant::now() + budget;
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(source) => {
                let _ = child.kill();
                return Err(SpawnFailure {
                    detail: format!("could not wait on `{bin}`: {source}"),
                    command,
                    reason: UnknownReason::SpawnFailed,
                });
            }
        }
        if Instant::now() >= deadline {
            timed_out = true;
            let _ = child.kill();
            break child.wait().ok();
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    // Best-effort, time-bounded: an orphan may still hold the write end.
    let drain_deadline = Instant::now() + PIPE_DRAIN_GRACE;
    let stdout = collect(&stdout_reader, drain_deadline);
    let stderr = collect(&stderr_reader, drain_deadline);

    Ok(Invocation {
        command,
        stdout,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        exit_code: if timed_out {
            None
        } else {
            status.and_then(|s| s.code())
        },
        timed_out,
    })
}

/// Cached `kicad-cli version` output. `None` means the probe failed; that
/// alone is not a blocking condition, it just leaves `tool_version`
/// unset in the evidence.
pub fn version() -> Option<String> {
    static VERSION: OnceLock<Option<String>> = OnceLock::new();
    VERSION.get_or_init(probe_version).clone()
}

fn probe_version() -> Option<String> {
    let run = run(
        &["version".to_string()],
        Duration::from_secs(VERSION_PROBE_TIMEOUT_SECS),
    )
    .ok()?;
    if !run.succeeded() {
        return None;
    }
    let text = String::from_utf8_lossy(&run.stdout);
    let line = text.lines().next()?.trim();
    (!line.is_empty()).then(|| line.to_string())
}

/// Parse the major version out of a `kicad-cli version` string such as
/// `10.0.1` or `8.0.4-unknown-abc123`.
pub fn major_version(version: &str) -> Option<u32> {
    let digits: String = version
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Reject tool versions whose report schema Synth cannot read.
///
/// Returns the reason and detail for an `unknown`, or `None` when the
/// version is acceptable or simply unknown — an unqueryable version is
/// not itself evidence of incompatibility.
pub fn version_rejection(version: Option<&str>) -> Option<(UnknownReason, String)> {
    let version = version?;
    let major = major_version(version)?;
    if major >= MIN_SUPPORTED_MAJOR {
        return None;
    }
    Some((
        UnknownReason::UnsupportedVersion,
        format!(
            "kicad-cli {version} is older than the minimum supported major \
             version {MIN_SUPPORTED_MAJOR}; its reports have no JSON schema Synth can read"
        ),
    ))
}

/// A temp path that cleans itself up, so an early return cannot leave a
/// report behind for a later run to mistake for its own output.
///
/// The old DRC path named its report by process id alone and removed it
/// only on the success path. A stale report from an aborted run could
/// then be read as the current run's result — a false clean in a
/// safety gate. The name here also carries a per-process counter so two
/// concurrent in-process runs cannot share a path.
#[derive(Debug)]
pub struct ScratchFile {
    path: std::path::PathBuf,
}

impl ScratchFile {
    /// Reserve a unique path under the temp dir. Nothing is created; the
    /// tool writes it. Any pre-existing file at the path is removed so a
    /// collision cannot be read as this run's output.
    pub fn reserve(prefix: &str, extension: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("{prefix}_{}_{seq}.{extension}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The path as a string for passing to `kicad-cli`.
    ///
    /// Lossy rather than `unwrap`: a non-UTF-8 temp dir used to panic
    /// here, and an export is not the place to abort the process.
    pub fn arg(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

impl Drop for ScratchFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_version_parses_kicad_formats() {
        assert_eq!(major_version("10.0.1"), Some(10));
        assert_eq!(major_version("8.0.4-unknown-abc123"), Some(8));
        assert_eq!(major_version("  9.0.0\n"), Some(9));
        assert_eq!(major_version("v10.0"), Some(10));
        assert_eq!(major_version("not a version"), None);
    }

    #[test]
    fn versions_below_the_minimum_are_rejected() {
        let (reason, detail) = version_rejection(Some("7.0.11")).expect("7.x is unsupported");
        assert_eq!(reason, UnknownReason::UnsupportedVersion);
        assert!(detail.contains("7.0.11"), "{detail}");
    }

    #[test]
    fn supported_and_future_versions_are_accepted() {
        assert!(version_rejection(Some("8.0.4")).is_none());
        assert!(version_rejection(Some("10.0.1")).is_none());
        // No upper bound: a future major must not read as unsupported.
        assert!(version_rejection(Some("14.2.0")).is_none());
    }

    #[test]
    fn an_unqueryable_version_is_not_a_rejection() {
        assert!(version_rejection(None).is_none());
        assert!(version_rejection(Some("mystery build")).is_none());
    }

    #[test]
    fn scratch_file_paths_are_unique_and_self_cleaning() {
        let a = ScratchFile::reserve("synth_test_scratch", "json");
        let b = ScratchFile::reserve("synth_test_scratch", "json");
        assert_ne!(a.path(), b.path());

        let path = a.path().to_path_buf();
        std::fs::write(&path, b"{}").expect("write scratch");
        assert!(path.exists());
        drop(a);
        assert!(
            !path.exists(),
            "scratch file must not outlive its guard: {}",
            path.display()
        );
    }

    #[test]
    fn reserving_clears_a_colliding_leftover() {
        let first = ScratchFile::reserve("synth_test_collide", "json");
        let path = first.path().to_path_buf();
        std::fs::write(&path, b"stale").expect("write stale report");
        // Forget the guard so the stale file survives, the way an
        // aborted run would leave it.
        std::mem::forget(first);
        assert!(path.exists());

        // A fresh reservation on the same path must not inherit it.
        let stale = std::fs::read(&path).unwrap();
        assert_eq!(stale, b"stale");
        let _ = std::fs::remove_file(&path);
    }

    /// The runner's own spawn/timeout/drain behaviour, exercised against
    /// stub programs rather than a real KiCad install so the classification
    /// is tested on every machine, including CI without KiCad.
    #[cfg(unix)]
    mod runner {
        use super::*;

        fn sh(script: &str, budget_ms: u64) -> Result<Invocation, SpawnFailure> {
            run_binary(
                "/bin/sh",
                &["-c".to_string(), script.to_string()],
                Duration::from_millis(budget_ms),
            )
        }

        #[test]
        fn a_missing_executable_is_not_installed() {
            let err = run_binary(
                "synth-definitely-not-a-real-kicad-cli",
                &[],
                Duration::from_secs(5),
            )
            .expect_err("a missing binary cannot run");
            assert_eq!(err.reason, UnknownReason::NotInstalled);
            assert_eq!(err.command[0], "synth-definitely-not-a-real-kicad-cli");
        }

        #[test]
        fn a_nonzero_exit_is_command_failed_and_keeps_stderr() {
            let run = sh("echo 'library not found' >&2; exit 3", 5_000).expect("stub ran");
            assert!(!run.succeeded());
            assert_eq!(run.exit_code, Some(3));
            assert_eq!(run.failure_reason(), Some(UnknownReason::CommandFailed));
            assert!(run.stderr.contains("library not found"), "{}", run.stderr);
            let detail = run.failure_detail(Duration::from_secs(5));
            assert!(detail.contains("code 3"), "{detail}");
        }

        #[test]
        fn a_clean_exit_succeeds_and_keeps_stdout() {
            let run = sh("echo 10.0.1", 5_000).expect("stub ran");
            assert!(run.succeeded());
            assert!(run.failure_reason().is_none());
            assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "10.0.1");
        }

        #[test]
        fn a_hung_tool_is_killed_and_reported_as_timeout() {
            let started = Instant::now();
            let run = sh("sleep 30", 300).expect("stub ran");
            assert!(run.timed_out);
            assert_eq!(run.failure_reason(), Some(UnknownReason::Timeout));
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "the budget must be enforced, not waited out: {:?}",
                started.elapsed()
            );
            let detail = run.failure_detail(Duration::from_millis(300));
            assert!(detail.contains("did not finish"), "{detail}");
        }

        /// Regression: a child that outsplashes one pipe buffer (64 KiB on
        /// Linux) blocks on write. A try_wait poll loop without draining
        /// readers would never see it exit, so this test hangs rather than
        /// fails if the readers are removed.
        #[test]
        fn a_chatty_tool_does_not_deadlock_the_poll_loop() {
            let run = sh(
                "i=0; while [ $i -lt 400 ]; do \
                   printf '%0.sx' $(seq 1 1000) >&2; i=$((i+1)); done; exit 1",
                20_000,
            )
            .expect("stub ran");
            assert!(!run.timed_out, "draining readers must prevent a deadlock");
            assert_eq!(run.failure_reason(), Some(UnknownReason::CommandFailed));
            assert!(
                run.stderr.len() > 64 * 1024,
                "expected more than one pipe buffer of stderr, got {}",
                run.stderr.len()
            );
        }
    }

    #[test]
    fn timeout_falls_back_on_a_zero_or_junk_override() {
        // Uses the real env, so only assert the default when unset.
        if std::env::var_os("SYNTH_KICAD_CLI_TIMEOUT_SECS").is_none() {
            assert_eq!(timeout(), Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        }
    }
}
