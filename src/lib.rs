//! c1ft: tests written in c1f, run on the cliffvm engine (docs/README.md).
pub mod cache;
pub mod format;
pub mod run;

/// The engine the cached results were made under: part of every cache key (tests/pin.rs keeps it equal to the release Cargo.toml pins).
pub const ENGINE: &str = "cliffvm 0.4.1";
/// The version of the file format and of the key: bumped when a directive changes what a test means.
pub const FORMAT: &str = "c1ft-format 1";
