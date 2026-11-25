//! The cache key of a test (docs/reactive.md): the engine, the test's own text, and the bytes of every file it loads. The same bytes give the same key; a change to any input gives another.
use c1ft::cache::{inputs, key};
use c1ft::format::parse;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("c1ft-cache-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("packs")).unwrap();
    std::fs::write(d.join("packs/a.c1f"), "PACK a 1\nThing.twice getter: this.n * 2\n").unwrap();
    std::fs::write(d.join("packs/b.c1f"), "PACK b 1\nThing.thrice getter: this.n * 3\n").unwrap();
    std::fs::write(d.join("x.packs"), "a\nb\n").unwrap();
    d
}
const T: &str = "-- test t\n-- load packs/a.c1f\n-- given Thing:a {\"n\": 2}\n-- reads Thing:a.twice = 4\n\n-- test u\n-- list x.packs packs\n-- absent Thing:zz\n";

#[test]
fn the_key_is_the_same_for_the_same_inputs_and_changes_with_each_input() {
    let d = dir("k");
    let ts = parse("t.c1ft", T).unwrap();
    let k0 = key(&ts[0], &d).unwrap();
    assert_eq!(k0, key(&ts[0], &d).unwrap(), "the same inputs, the same key");
    assert_eq!(k0.len(), 64, "a sha-256 in hex: {k0}");
    assert_ne!(k0, key(&ts[1], &d).unwrap(), "another test, another key");
    // a file the test loads changes
    std::fs::write(d.join("packs/a.c1f"), "PACK a 1\nThing.twice getter: this.n * 20\n").unwrap();
    assert_ne!(k0, key(&ts[0], &d).unwrap(), "a loaded file changed");
    // a file it does not load does not matter
    let k1 = key(&ts[0], &d).unwrap();
    std::fs::write(d.join("packs/b.c1f"), "PACK b 1\nThing.thrice getter: this.n * 30\n").unwrap();
    assert_eq!(k1, key(&ts[0], &d).unwrap(), "t does not load b");
    // a list: the list file and every pack it names are inputs
    let ku = key(&ts[1], &d).unwrap();
    std::fs::write(d.join("packs/b.c1f"), "PACK b 1\n").unwrap();
    assert_ne!(ku, key(&ts[1], &d).unwrap(), "a pack of the list changed");
    let ku = key(&ts[1], &d).unwrap();
    std::fs::write(d.join("x.packs"), "a\n").unwrap();
    assert_ne!(ku, key(&ts[1], &d).unwrap(), "the list changed");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_change_to_another_test_of_the_same_file_leaves_the_key_alone() {
    let d = dir("o");
    let a = parse("t.c1ft", T).unwrap();
    let edited = T.replace("absent Thing:zz", "absent Thing:yy");
    let b = parse("t.c1ft", &edited).unwrap();
    assert_eq!(key(&a[0], &d).unwrap(), key(&b[0], &d).unwrap());
    assert_ne!(key(&a[1], &d).unwrap(), key(&b[1], &d).unwrap());
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_folder_is_an_input_by_every_file_in_it_and_the_inputs_are_listed_for_the_watcher() {
    let d = dir("f");
    std::fs::create_dir_all(d.join("app/sub")).unwrap();
    std::fs::write(d.join("app/one.c1f"), "PACK one 1\n").unwrap();
    std::fs::write(d.join("app/sub/two.md"), "text").unwrap();
    let ts = parse("t.c1ft", "-- test t\n-- load app\n").unwrap();
    let k = key(&ts[0], &d).unwrap();
    std::fs::write(d.join("app/sub/two.md"), "text changed").unwrap();
    assert_ne!(k, key(&ts[0], &d).unwrap(), "a file deep in a loaded folder");
    let mut names: Vec<String> = inputs(&ts[0], &d).unwrap().iter().map(|p| p.strip_prefix(&d).unwrap().to_string_lossy().to_string()).collect();
    names.sort();
    assert_eq!(names, ["app/one.c1f", "app/sub/two.md"]);
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn an_input_that_is_not_there_is_an_error_that_names_it_not_a_silent_key() {
    let d = dir("m");
    let ts = parse("t.c1ft", "-- test t\n-- load packs/missing.c1f\n").unwrap();
    let e = key(&ts[0], &d).unwrap_err();
    assert!(e.contains("packs/missing.c1f"), "{e}");
    let _ = std::fs::remove_dir_all(d);
}
