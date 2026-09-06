//! Guards the crate's public API surface.
//!
//! A file directly under `tests/` compiles as a separate, external crate,
//! so it only sees what `lib.rs` actually re-exports — unlike a unit test
//! inside `src/`, which sees every private item regardless of what is
//! re-exported. This is what catches a regression a unit test cannot:
//! `MarkerSpec` was once declared `pub` inside the private `ecosystem`
//! module but never added to `lib.rs`'s `pub use ecosystem::{...}` list,
//! so `Markers.must`/`.should` (`Vec<MarkerSpec>`) were externally visible
//! and iterable, but `MarkerSpec` itself had no path any external crate
//! could name — `use repofolio_core::MarkerSpec` failed with E0432, and
//! nothing outside the crate could construct one or match `One`/`AnyOf`
//! by name.

use repofolio_core::{MarkerSpec, Markers};

#[test]
fn marker_spec_is_constructible_and_matchable_by_name_from_outside_the_crate() {
    let markers = Markers {
        must: vec![MarkerSpec::One("Cargo.toml".to_string())],
        should: vec![MarkerSpec::AnyOf(vec![
            "rustfmt.toml".to_string(),
            ".rustfmt.toml".to_string(),
        ])],
    };

    match &markers.must[0] {
        MarkerSpec::One(path) => assert_eq!(path, "Cargo.toml"),
        MarkerSpec::AnyOf(_) => panic!("expected MarkerSpec::One"),
    }

    match &markers.should[0] {
        MarkerSpec::AnyOf(paths) => {
            assert_eq!(
                paths,
                &vec!["rustfmt.toml".to_string(), ".rustfmt.toml".to_string()]
            );
        }
        MarkerSpec::One(_) => panic!("expected MarkerSpec::AnyOf"),
    }
}
