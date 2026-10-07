// SPDX-License-Identifier: Apache-2.0

//! A stub external router for the CLI integration tests.
//!
//! The release gate is fail-closed on routing: an export with no
//! independently validated route is not fabricable. That means a test that
//! wants to assert anything about the *rest* of the gate — ERC evidence, DRC
//! evidence, the release manifest — has to supply a route, or it will only
//! ever see the routing gate refuse.
//!
//! Installing a stub keeps those tests honest: the pipeline runs its real
//! export, real adapter invocation, and real independent checks, and only
//! the engine's copper is a stand-in.

#![cfg(unix)]
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Environment this helper installs.
///
/// Restored on drop. The adapters read `PATH` to find the interpreter that
/// runs the routing helper, and the router environment variables to find
/// the engine, so a stub has to reach the process environment rather than a
/// struct.
pub struct StubRouter {
    bin_dir: PathBuf,
    previous_path: Option<std::ffi::OsString>,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

/// Install the stub router for the duration of one test.
///
/// `out_dir` receives the stub JAR and Java stand-ins: the adapter insists
/// both exist before it will start the helper, so they have to be present.
pub fn install(out_dir: &Path) -> StubRouter {
    let bin_dir = out_dir.join("stub-router-bin");
    std::fs::create_dir_all(&bin_dir).expect("stub bin dir");

    // The stub is installed as `python3`, so the adapter invokes it exactly as
    // it invokes the real helper: as the interpreter. Its shebang is rewritten
    // to the absolute path of a real interpreter found *before* this directory
    // goes on PATH, so it cannot resolve back to itself.
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("stub_router.py");
    let real_python = real_interpreter();
    let body = std::fs::read_to_string(&source).expect("read stub router");
    let interpreter = bin_dir.join("python3");
    std::fs::write(
        &interpreter,
        body.replacen(
            "#!/usr/bin/python3",
            &format!("#!{}", real_python.display()),
            1,
        ),
    )
    .expect("write stub router");
    make_executable(&interpreter);

    let jar = out_dir.join("tools/freerouting/freerouting-stub.jar");
    if let Some(parent) = jar.parent() {
        std::fs::create_dir_all(parent).expect("tools dir");
    }
    std::fs::write(&jar, b"stub jar").expect("write stub jar");
    let java = out_dir.join("tools/jre25/bin/java");
    if let Some(parent) = java.parent() {
        std::fs::create_dir_all(parent).expect("jre dir");
    }
    std::fs::write(&java, b"stub java").expect("write stub java");

    let previous_path = std::env::var_os("PATH");
    let joined = match &previous_path {
        Some(existing) => {
            use std::os::unix::ffi::OsStrExt as _;
            format!(
                "{}:{}",
                bin_dir.display(),
                existing.as_bytes().escape_ascii()
            )
        }
        None => bin_dir.display().to_string(),
    };
    std::env::set_var("PATH", joined);

    let mut previous = Vec::new();
    for (key, value) in [
        ("SYNTH_FREEROUTING_JAR", jar.into_os_string()),
        ("SYNTH_FREEROUTING_JAVA", java.into_os_string()),
    ] {
        let saved = std::env::var_os(key);
        std::env::set_var(key, value);
        previous.push((key, saved));
    }

    StubRouter {
        bin_dir,
        previous_path,
        previous,
    }
}

/// Locate a real Python interpreter, ignoring this helper's own directory.
///
/// Called before the stub bin directory is prepended to `PATH`, so the first
/// hit is never the stub itself.
fn real_interpreter() -> PathBuf {
    for name in ["python3", "python"] {
        let found = Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {name}"))
            .output()
            .expect("probe PATH");
        if found.status.success() {
            let path = String::from_utf8_lossy(&found.stdout).trim().to_string();
            if !path.is_empty() {
                return PathBuf::from(path);
            }
        }
    }
    panic!("no python3 on PATH to run the stub router");
}

fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path).expect("stat stub").permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).expect("chmod stub");
}

impl StubRouter {
    /// The directory the stub interpreter lives in.
    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }
}

impl Drop for StubRouter {
    fn drop(&mut self) {
        for (key, value) in &self.previous {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        match &self.previous_path {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
    }
}

/// Serialises the tests that mutate the process environment.
///
/// Cargo runs tests on parallel threads in one process, so without this two
/// tests would fight over `PATH` and each would see the other's stub.
pub fn serialised() -> std::sync::MutexGuard<'static, ()> {
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Run `synth` with the stub router installed.
///
/// Convenience for the many gate tests whose only difference is the stub
/// `kicad-cli` they need.
pub fn synth_with_stub_router(synth: &str, scratch: &Path, args: &[&str]) -> std::process::Output {
    let _lock = serialised();
    let _router = install(scratch);
    Command::new(synth).args(args).output().expect("run synth")
}
