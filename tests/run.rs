//! Running one test: a fresh engine per test (no store, no effects), the loads first, then the test's own pack, then the steps in order.
use c1ft::format::parse;
use c1ft::run::run_test;
use std::path::Path;

fn one(text: &str) -> Result<(), String> {
    let ts = parse("t.c1ft", text).unwrap();
    run_test(&ts[0], Path::new("."))
}

const BASE: &str = "-- test t
PACK p 1
Thing.twice getter: this.n * 2
Thing.label getter: 'n=' + str(this.n)
";

#[test]
fn a_test_that_reads_what_it_says_passes() {
    one(&format!("{BASE}-- given Thing:a {{\"n\": 2}}\n-- reads Thing:a.twice = 4\n-- reads Thing:a.label = \"n=2\"\n")).unwrap();
}

#[test]
fn a_write_changes_what_a_getter_reads_and_an_absent_property_reads_null() {
    one(&format!("{BASE}-- given Thing:a {{\"n\": 2}}\n-- write Thing:a.n = 5\n-- reads Thing:a.twice = 10\n-- reads Thing:a.nothing = null\n")).unwrap();
}

#[test]
fn a_number_compares_by_value_not_by_how_it_was_written() {
    one(&format!("{BASE}-- given Thing:a {{\"n\": 2}}\n-- reads Thing:a.twice = 4.0\n")).unwrap();
}

#[test]
fn a_wrong_read_fails_and_says_where_what_was_read_and_what_was_expected() {
    let e = one(&format!("{BASE}-- given Thing:a {{\"n\": 2}}\n-- reads Thing:a.twice = 5\n")).unwrap_err();
    assert!(e.contains("t.c1ft:6") && e.contains("Thing:a.twice") && e.contains("reads 4") && e.contains("expected 5"), "{e}");
}

#[test]
fn exists_and_absent_check_the_entity() {
    one(&format!("{BASE}-- given Thing:a {{\"n\": 2}}\n-- exists Thing:a\n-- absent Thing:b\n")).unwrap();
    let e = one(&format!("{BASE}-- absent Thing:a\n-- given Thing:a {{\"n\": 2}}\n-- absent Thing:a\n")).unwrap_err();
    assert!(e.contains("t.c1ft:7") && e.contains("Thing:a") && e.contains("exists"), "{e}");
    let e = one(&format!("{BASE}-- exists Thing:zz\n")).unwrap_err();
    assert!(e.contains("Thing:zz") && e.contains("does not exist"), "{e}");
}

#[test]
fn a_prefixed_id_is_born_with_the_class_after_the_prefix() {
    one("-- test t\nPACK p 1\nThing.twice getter: this.n * 2\n-- given ns::Thing:a {\"n\": 3}\n-- reads ns::Thing:a.twice = 6\n").unwrap();
}

#[test]
fn a_pack_the_loader_refuses_fails_the_test_unless_it_expects_the_refusal() {
    let bad = "-- test t\nPACK p 1\nThing.x getter: this.n +\n";
    let e = one(bad).unwrap_err();
    assert!(e.contains("pack p"), "{e}");
    let ok = "-- test t\nPACK p 1\nThing.x getter: this.n +\n-- error: pack p\n";
    one(ok).unwrap();
    let e = one("-- test t\nPACK p 1\nThing.x getter: 1\n-- error: whatever\n").unwrap_err();
    assert!(e.contains("accepted"), "{e}");
    let e = one("-- test t\nPACK p 1\nThing.x getter: this.n +\n-- error: a text the loader never says\n").unwrap_err();
    assert!(e.contains("a text the loader never says"), "{e}");
}

#[test]
fn a_test_with_no_pack_still_runs_its_steps_on_an_empty_engine() {
    one("-- test t\n-- absent Thing:a\n").unwrap();
}

#[test]
fn every_test_gets_its_own_engine() {
    let ts = parse("t.c1ft", &format!("{BASE}-- given Thing:a {{\"n\": 1}}\n\n-- test u\n-- absent Thing:a\n")).unwrap();
    run_test(&ts[0], Path::new(".")).unwrap();
    run_test(&ts[1], Path::new(".")).unwrap();
}

#[test]
fn a_pack_file_is_loaded_relative_to_the_base_and_a_list_loads_its_packs_first_one_trusted() {
    let d = std::env::temp_dir().join(format!("c1ft-run-{}", std::process::id()));
    std::fs::create_dir_all(d.join("packs")).unwrap();
    std::fs::write(d.join("packs/a.c1f"), "PACK a 1\nThing.twice getter: this.n * 2\n").unwrap();
    std::fs::write(d.join("packs/b.c1f"), "PACK b 1\nOther.thrice getter: this.n * 3\n").unwrap();
    std::fs::write(d.join("x.packs"), "# the packs, in order\na\nb\n").unwrap();
    let ts = parse("t.c1ft", "-- test t\n-- load packs/a.c1f\n-- given Thing:a {\"n\": 2}\n-- reads Thing:a.twice = 4\n\n-- test u\n-- list x.packs packs\n-- given Thing:a {\"n\": 2}\n-- given Other:o {\"n\": 2}\n-- reads Other:o.thrice = 6\n-- reads Thing:a.twice = 4\n").unwrap();
    run_test(&ts[0], &d).unwrap();
    run_test(&ts[1], &d).unwrap();
    let ts = parse("t.c1ft", "-- test t\n-- load packs/missing.c1f\n").unwrap();
    let e = run_test(&ts[0], &d).unwrap_err();
    assert!(e.contains("packs/missing.c1f"), "{e}");
    let _ = std::fs::remove_dir_all(d);
}

#[test]
fn a_loaded_pack_the_loader_refuses_fails_the_test_and_says_why() {
    let d = std::env::temp_dir().join(format!("c1ft-run-refused-{}", std::process::id()));
    std::fs::create_dir_all(d.join("packs")).unwrap();
    std::fs::write(d.join("packs/bad.c1f"), "PACK bad 1\nThing.x getter: this.n +\n").unwrap();
    std::fs::write(d.join("x.packs"), "bad\n").unwrap();
    for text in ["-- test t\n-- load packs/bad.c1f\n", "-- test t\n-- list x.packs packs\n"] {
        let ts = parse("t.c1ft", text).unwrap();
        let e = run_test(&ts[0], &d).unwrap_err();
        assert!(e.contains("t.c1ft:1") && e.contains("bad"), "a refused pack must not pass silently: {e}");
    }
    let _ = std::fs::remove_dir_all(d);
}
