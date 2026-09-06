//! Guards CLAUDE.md's architecture decision #2 ("Sync only, everywhere
//! except `mcp`") at the dependency-graph level: no crate in this
//! workspace has an `mcp` subcommand yet, so no *normal* (non-dev,
//! non-build) dependency edge should ever resolve to `tokio` or
//! `reqwest`. `jsonschema`'s default features pulled both in
//! transitively through `repofolio-manifest` until the root `Cargo.toml`
//! disabled them (`default-features = false`) — this test fails on that
//! exact regression, and fails again if it recurs (a `jsonschema` bump
//! that re-enables a resolver feature, or an unrelated dependency
//! bump that reintroduces the same transitive edge).

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/repofolio has two parent directories")
        .to_path_buf()
}

/// Confirms `crate_name` itself resolves in `cargo tree`, independent of
/// any `-i` filter. `cargo tree -p <crate> -e normal -i <dep>` reports
/// "did not match any packages" identically whether `<crate>` or `<dep>`
/// is the one that failed to resolve, so this must be checked separately
/// — otherwise a typo'd crate name in this test would be misread as
/// proof that `dep` is absent, rather than the test never having run at
/// all.
fn crate_resolves(crate_name: &str) {
    let output = Command::new("cargo")
        .args(["tree", "-p", crate_name, "-e", "normal"])
        .current_dir(workspace_root())
        .output()
        .expect("run cargo tree");
    assert!(
        output.status.success(),
        "cargo tree -p {crate_name} failed to resolve — is {crate_name} a real \
         workspace member? stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `cargo tree -e normal -i <dep>` exits non-zero ("did not match any
/// packages") when `dep` is absent from `crate_name`'s resolved normal
/// dependency graph, and exits 0 (printing the dependency path) when
/// present. A non-zero exit for any *other* reason — a stale lock file,
/// an unrelated unsatisfiable version requirement, a transient registry
/// error — must not be read as "absent": that would silently report the
/// sync-only invariant as upheld without having checked it at all, so
/// this asserts the failure is specifically the "not found" one before
/// treating it as a negative result.
fn normal_dependency_graph_contains(crate_name: &str, dep: &str) -> bool {
    let output = Command::new("cargo")
        .args(["tree", "-p", crate_name, "-e", "normal", "-i", dep])
        .current_dir(workspace_root())
        .output()
        .expect("run cargo tree");

    if output.status.success() {
        return true;
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("did not match any packages"),
        "cargo tree -p {crate_name} -e normal -i {dep} failed for an unexpected \
         reason, not because {dep} is absent: {stderr}"
    );
    false
}

#[test]
fn no_workspace_crate_pulls_in_tokio_or_reqwest_as_a_normal_dependency() {
    for crate_name in [
        "repofolio",
        "repofolio-core",
        "repofolio-manifest",
        "repofolio-templates",
        "repofolio-tools",
        "repofolio-fmt",
        "repofolio-release",
    ] {
        crate_resolves(crate_name);
        for dep in ["tokio", "reqwest"] {
            assert!(
                !normal_dependency_graph_contains(crate_name, dep),
                "{crate_name} pulls in {dep} as a normal dependency, violating \
                 CLAUDE.md's sync-only architecture decision (none of these \
                 crates has an mcp subcommand)"
            );
        }
    }
}
