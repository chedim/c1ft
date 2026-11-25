//! The program: `c1ft run | list | watch` (docs/runner.md).
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str { env!("CARGO_BIN_EXE_c1ft") }
fn fixtures() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures") }
fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("c1ft-cli-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}
fn run(args: &[&str]) -> Output { Command::new(bin()).args(args).output().unwrap() }
fn out(o: &Output) -> String { format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)) }

#[test]
fn passing_tests_exit_zero_and_say_so_and_a_second_run_takes_them_from_the_cache() {
    let c = tmp("pass");
    let dir = fixtures().join("ok");
    let o = run(&["run", dir.to_str().unwrap(), "--cache-dir", c.to_str().unwrap()]);
    let t = out(&o);
    assert!(o.status.success(), "{t}");
    assert!(t.contains("PASS") && t.contains("ok.c1ft: doubles") && t.contains("2 tests: 2 passed (0 cached), 0 failed"), "{t}");
    let o = run(&["run", dir.to_str().unwrap(), "--cache-dir", c.to_str().unwrap()]);
    let t = out(&o);
    assert!(o.status.success() && t.contains("CACHED") && t.contains("2 tests: 2 passed (2 cached), 0 failed"), "{t}");
    let o = run(&["run", dir.to_str().unwrap(), "--cache-dir", c.to_str().unwrap(), "--no-cache"]);
    let t = out(&o);
    assert!(t.contains("2 passed (0 cached)") && !t.contains("CACHED"), "--no-cache runs everything: {t}");
    let _ = std::fs::remove_dir_all(c);
}

#[test]
fn a_failing_test_exits_one_with_where_and_why_and_is_never_taken_from_the_cache() {
    let c = tmp("fail");
    let dir = fixtures().join("bad");
    for _ in 0..2 {
        let o = run(&["run", dir.to_str().unwrap(), "--cache-dir", c.to_str().unwrap()]);
        let t = out(&o);
        assert_eq!(o.status.code(), Some(1), "{t}");
        assert!(t.contains("FAIL") && t.contains("bad.c1ft: fails") && t.contains("bad.c1ft:11") && t.contains("expected 5"), "{t}");
        assert!(t.contains("2 tests: 1 passed") && t.contains("1 failed"), "{t}");
    }
    let o = run(&["run", dir.to_str().unwrap(), "--cache-dir", c.to_str().unwrap()]);
    assert!(out(&o).contains("(1 cached)"), "the pass is cached, the failure ran again: {}", out(&o));
    let _ = std::fs::remove_dir_all(c);
}

#[test]
fn a_filter_runs_the_tests_whose_name_holds_the_text() {
    let c = tmp("filter");
    let dir = fixtures().join("ok");
    let o = run(&["run", dir.to_str().unwrap(), "--cache-dir", c.to_str().unwrap(), "--filter", "follows"]);
    let t = out(&o);
    assert!(o.status.success() && t.contains("1 tests: 1 passed") && t.contains("follows a write") && !t.contains("doubles"), "{t}");
    let o = run(&["run", dir.to_str().unwrap(), "--cache-dir", c.to_str().unwrap(), "--filter", "nothing-like-it"]);
    assert_eq!(o.status.code(), Some(2), "a filter that matches no test is a mistake, not a pass: {}", out(&o));
    let _ = std::fs::remove_dir_all(c);
}

#[test]
fn list_names_every_test_with_its_file_and_runs_nothing() {
    let o = run(&["list", fixtures().join("bad").to_str().unwrap()]);
    let t = out(&o);
    assert!(o.status.success() && t.contains("bad.c1ft: passes") && t.contains("bad.c1ft: fails") && !t.contains("PASS"), "{t}");
}

#[test]
fn a_file_that_does_not_parse_is_exit_two_and_names_the_line() {
    let c = tmp("broken");
    let o = run(&["run", fixtures().join("broken").to_str().unwrap(), "--cache-dir", c.to_str().unwrap()]);
    let t = out(&o);
    assert_eq!(o.status.code(), Some(2), "{t}");
    assert!(t.contains("broken.c1ft:2") && t.contains("reads"), "{t}");
    let _ = std::fs::remove_dir_all(c);
}

#[test]
fn usage_mistakes_say_what_was_meant() {
    for (args, want) in [(vec![], "usage"), (vec!["frobnicate"], "unknown command"), (vec!["run", "--cache-dir"], "--cache-dir needs"), (vec!["watch"], "--every-ms"), (vec!["watch", "--every-ms", "soon"], "--every-ms")] {
        let o = run(&args);
        let t = out(&o);
        assert_eq!(o.status.code(), Some(2), "{args:?}: {t}");
        assert!(t.contains(want), "{args:?}: {t}");
    }
}

#[test]
fn watch_runs_again_when_an_input_changes_and_only_the_test_that_reads_it_runs() {
    let d = tmp("watch");
    std::fs::create_dir_all(d.join("packs")).unwrap();
    std::fs::write(d.join("packs/a.c1f"), "PACK a 1\nThing.twice getter: this.n * 2\n").unwrap();
    std::fs::write(d.join("packs/b.c1f"), "PACK b 1\nThing.thrice getter: this.n * 3\n").unwrap();
    std::fs::write(d.join("w.c1ft"), "-- test reads a\n-- load packs/a.c1f\n-- given Thing:a {\"n\": 2}\n-- reads Thing:a.twice = 4\n\n-- test reads b\n-- load packs/b.c1f\n-- given Thing:a {\"n\": 2}\n-- reads Thing:a.thrice = 6\n").unwrap();
    let child = Command::new(bin()).args(["watch", "--every-ms", "50", "--rounds", "2", "--cache-dir", d.join("cache").to_str().unwrap(), d.to_str().unwrap()]).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
    // the first round is done when the cache holds a pass for both tests
    let t0 = std::time::Instant::now();
    while std::fs::read_dir(d.join("cache")).map(|r| r.count()).unwrap_or(0) < 2 { assert!(t0.elapsed().as_secs() < 60, "the first round never finished"); std::thread::sleep(std::time::Duration::from_millis(50)); }
    std::thread::sleep(std::time::Duration::from_millis(300));
    std::fs::write(d.join("packs/a.c1f"), "PACK a 1\nThing.twice getter: this.n * 2 + 0\n").unwrap();
    let o = child.wait_with_output().unwrap();
    let t = out(&o);
    assert!(o.status.success(), "{t}");
    assert_eq!(t.matches(" tests: ").count(), 2, "two rounds: {t}");
    let first_end = t.find(" tests: ").map(|i| i + t[i..].find('\n').unwrap_or(0)).unwrap();
    let (r1, r2) = t.split_at(first_end);
    assert!(r1.contains("PASS   w.c1ft: reads a") && r1.contains("PASS   w.c1ft: reads b"), "the first round ran both: {t}");
    assert!(r2.contains("PASS   w.c1ft: reads a") && r2.contains("CACHED w.c1ft: reads b"), "the second ran the test that reads the changed pack and took the other from the cache: {t}");
    let _ = std::fs::remove_dir_all(d);
}
