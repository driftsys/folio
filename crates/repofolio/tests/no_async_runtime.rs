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

/// `cargo tree -e normal -i <dep>` exits non-zero ("did not match any
/// packages") when `dep` is absent from `crate_name`'s resolved normal
/// dependency graph, and exits 0 (printing the dependency path) when
/// present.
fn normal_dependency_graph_contains(crate_name: &str, dep: &str) -> bool {
    Command::new("cargo")
        .args(["tree", "-p", crate_name, "-e", "normal", "-i", dep])
        .current_dir(workspace_root())
        .output()
        .expect("run cargo tree")
        .status
        .success()
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
