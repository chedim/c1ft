//! The file format (docs/format.md): `-- test <name>` starts a test, the lines of a test that are not directives are its pack, the directives are the steps.
use c1ft::format::{parse, Load, Step};
use serde_json::json;

const TWO: &str = "-- a comment before any test is fine

-- test first
PACK p 1
Thing.twice getter: this.n * 2
-- load lib/helpers.c1f
-- list apps/x.packs packs
-- given Thing:a {\"n\": 2}
-- write Thing:a.n = 5
-- pump
-- reads Thing:a.twice = 10
-- exists Thing:a
-- absent Thing:nope

-- test second
PACK q 1
-- error: no such class
";

#[test]
fn a_file_is_a_list_of_named_tests_with_their_pack_loads_and_steps_in_order() {
    let ts = parse("f.c1ft", TWO).unwrap();
    assert_eq!(ts.len(), 2);
    let t = &ts[0];
    assert_eq!(t.name, "first");
    assert_eq!(t.line, 3);
    assert!(t.pack.starts_with("PACK p 1\nThing.twice getter: this.n * 2"), "{:?}", t.pack);
    assert_eq!(t.loads, vec![Load::File("lib/helpers.c1f".into()), Load::List { file: "apps/x.packs".into(), dir: "packs".into() }]);
    let kinds: Vec<&str> = t.steps.iter().map(|s| match &s.step { Step::Given { .. } => "given", Step::Write { .. } => "write", Step::Pump => "pump", Step::Reads { .. } => "reads", Step::Exists(_) => "exists", Step::Absent(_) => "absent" }).collect();
    assert_eq!(kinds, ["given", "write", "pump", "reads", "exists", "absent"]);
    assert!(matches!(&t.steps[0].step, Step::Given { eid, props } if eid == "Thing:a" && *props == json!({"n": 2})));
    assert!(matches!(&t.steps[1].step, Step::Write { eid, prop, value } if eid == "Thing:a" && prop == "n" && *value == json!(5)));
    assert!(matches!(&t.steps[3].step, Step::Reads { eid, prop, want } if eid == "Thing:a" && prop == "twice" && *want == json!(10)));
    assert_eq!(t.steps[3].line, 11, "a step knows its line in the file");
    assert_eq!(ts[1].name, "second");
    assert_eq!(ts[1].error.as_deref(), Some("no such class"));
}

#[test]
fn the_text_of_a_test_is_its_own_lines_so_a_change_elsewhere_in_the_file_leaves_it_alone() {
    let ts = parse("f.c1ft", TWO).unwrap();
    assert!(ts[0].text.contains("-- test first") && ts[0].text.contains("-- absent Thing:nope") && !ts[0].text.contains("second"));
    assert!(ts[1].text.contains("-- test second") && !ts[1].text.contains("first"));
}

#[test]
fn a_property_may_hold_a_dot_and_the_last_dot_splits_the_target() {
    let ts = parse("f.c1ft", "-- test t\n-- reads Thing:a.b.c = 1\n").unwrap();
    assert!(matches!(&ts[0].steps[0].step, Step::Reads { eid, prop, .. } if eid == "Thing:a.b" && prop == "c"));
}

#[test]
fn a_line_that_is_not_understood_names_the_file_and_the_line() {
    for (text, want) in [
        ("PACK p 1\n", "f.c1ft:1: a line outside a test"),
        ("-- test t\n-- given Thing:a not json\n", "f.c1ft:2: given: the properties are not JSON"),
        ("-- test t\n-- reads Thing:a = 1\n", "f.c1ft:2: reads: `<eid>.<prop> = <json>`"),
        ("-- test t\n-- write Thing:a.n\n", "f.c1ft:2: write: `<eid>.<prop> = <json>`"),
        ("-- test t\n-- list only-one\n", "f.c1ft:2: list: `<list file> <dir>`"),
        ("-- test \n", "f.c1ft:1: test: a name"),
        ("-- test t\n-- test t\n", "f.c1ft:2: a second test named t"),
    ] {
        let e = parse("f.c1ft", text).unwrap_err();
        assert!(e.contains(want), "{text:?}: {e}");
    }
}
