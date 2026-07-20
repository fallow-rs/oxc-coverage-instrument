//! Shared helpers for the `oxc-coverage-instrument` CLI integration tests.
//!
//! `CARGO_BIN_EXE_oxc-coverage-instrument` is set by cargo for integration tests
//! of binary packages, so the binary is driven end to end through
//! `std::process::Command` with no extra dev-dependency.

use std::{env, fs, path::PathBuf, process::Command};

pub fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_oxc-coverage-instrument"))
}

/// Build a path under the system temp directory, scoped to this process so two
/// concurrent runs on one machine cannot delete each other's trees mid-assertion.
pub fn temp_path(name: &str) -> PathBuf {
    env::temp_dir().join(format!("oxc_cov_cli_{}_{name}", std::process::id()))
}

pub fn write_temp(name: &str, contents: &str) -> PathBuf {
    let path = temp_path(name);
    fs::write(&path, contents).unwrap();
    path
}
