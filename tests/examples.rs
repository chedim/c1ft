//! The examples (examples/) are real tests of the engine: they pass, and they fail when what they check is wrong.
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf() }

#[test]
fn the_bare_twin_example_passes() {
    let c = std::env::temp_dir().join(format!("c1ft-ex-{}", std::process::id()));
    let o = Command::new(env!("CARGO_BIN_EXE_c1ft")).args(["run", "--no-cache", "--cache-dir", c.to_str().unwrap(), root().join("examples/bare_twin").to_str().unwrap()]).output().unwrap();
    let t = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success() && t.contains("7 tests: 7 passed"), "{t}");
    let _ = std::fs::remove_dir_all(c);
}

#[test]
fn an_example_whose_expectation_is_wrong_fails() {
    let d = std::env::temp_dir().join(format!("c1ft-ex-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::copy(root().join("examples/bare_twin/bt.c1f"), d.join("bt.c1f")).unwrap();
    let text = std::fs::read_to_string(root().join("examples/bare_twin/bare_twin.c1ft")).unwrap().replace("-- reads Reader:r3.one = 9", "-- reads Reader:r3.one = 4");
    std::fs::write(d.join("bare_twin.c1ft"), text).unwrap();
    let o = Command::new(env!("CARGO_BIN_EXE_c1ft")).args(["run", "--no-cache", "--cache-dir", d.join("cache").to_str().unwrap(), d.to_str().unwrap()]).output().unwrap();
    let t = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert_eq!(o.status.code(), Some(1), "{t}");
    assert!(t.contains("the bare entity wins when both exist") && t.contains("reads 9, expected 4") && t.contains("6 passed") && t.contains("1 failed"), "{t}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn the_eval_example_passes() {
    let c = std::env::temp_dir().join(format!("c1ft-ex-eval-{}", std::process::id()));
    let o = Command::new(env!("CARGO_BIN_EXE_c1ft")).args(["run", "--no-cache", "--cache-dir", c.to_str().unwrap(), root().join("examples/eval").to_str().unwrap()]).output().unwrap();
    let t = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(o.status.success() && t.contains("3 tests: 3 passed"), "{t}");
    let _ = std::fs::remove_dir_all(c);
}
