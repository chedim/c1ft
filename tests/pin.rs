//! The engine the results are cached under (c1ft::ENGINE) is the release Cargo.toml pins.
#[test]
fn the_engine_constant_is_the_pinned_release() {
    let toml = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let pin = toml.lines().find(|l| l.starts_with("cliffvm = ")).and_then(|l| l.split("\"=").nth(1)).and_then(|r| r.split('"').next()).expect("the cliffvm pin");
    assert_eq!(c1ft::ENGINE, format!("cliffvm {pin}"));
}
