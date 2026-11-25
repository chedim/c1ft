//! A directive may start its line with the keyword itself: `given Thing:a {...}` is `-- given Thing:a {...}`. The `-- ` form stays valid, and both may share a file.
use c1ft::format::{parse, Load, Step};
use c1ft::run::run_test;
use std::path::Path;

const BARE: &str = "test first
PACK p 1
Thing.twice getter: this.n * 2
load lib/helpers.c1f
list apps/x.packs packs
given Thing:a {\"n\": 2}
write Thing:a.n = 5
pump
reads Thing:a.twice = 10
exists Thing:a
absent Thing:nope
eval Thing:a.twice + 1 = 11

test second
PACK q 1
error: no such class
";

#[test]
fn bare_keywords_parse_to_the_same_steps_as_the_dashed_form() {
    let dashed: String = BARE.lines().map(|l| {
        let w = l.split_whitespace().next().unwrap_or("");
        if ["test", "load", "list", "given", "write", "pump", "reads", "exists", "absent", "eval", "error:"].contains(&w) { format!("-- {l}\n") } else { format!("{l}\n") }
    }).collect();
    let a = parse("f.c1ft", BARE).unwrap();
    let b = parse("f.c1ft", &dashed).unwrap();
    assert_eq!(a.len(), 2);
    for (x, y) in a.iter().zip(&b) {
        assert_eq!((&x.name, x.line, &x.pack, &x.loads, &x.steps, &x.error), (&y.name, y.line, &y.pack, &y.loads, &y.steps, &y.error));
    }
    assert_eq!(a[0].loads, vec![Load::File("lib/helpers.c1f".into()), Load::List { file: "apps/x.packs".into(), dir: "packs".into() }]);
    assert!(a[0].steps.iter().any(|s| matches!(&s.step, Step::Eval { .. })) && a[0].steps.iter().any(|s| matches!(&s.step, Step::Pump)));
    assert_eq!(a[1].error.as_deref(), Some("no such class"));
}

#[test]
fn the_two_forms_may_share_a_file_and_a_comment_stays_a_comment() {
    let ts = parse("f.c1ft", "-- a comment before the first test\n\ntest t\n-- a comment inside it\nPACK p 1\nThing.twice getter: this.n * 2\n-- given Thing:a {\"n\": 2}\nreads Thing:a.twice = 4\n").unwrap();
    assert_eq!(ts[0].steps.len(), 2);
    assert!(ts[0].pack.contains("-- a comment inside it"));
}

#[test]
fn a_pack_line_is_never_taken_for_a_directive() {
    // the pack's own lines start with PACK, IMPORT or a class name, and a body line is indented: none starts with a keyword
    let ts = parse("f.c1ft", "test t\nPACK p 1\nThing.list getter: [1]\nThing.given getter:\n  reads = 1\n  reads\ngiven Thing:a {}\n").unwrap();
    assert_eq!(ts[0].steps.len(), 1, "{:?}", ts[0].steps);
    assert!(ts[0].pack.contains("Thing.list getter: [1]") && ts[0].pack.contains("  reads = 1") && ts[0].pack.contains("  reads\n"), "{:?}", ts[0].pack);
}

#[test]
fn a_bare_test_runs() {
    let ts = parse("t.c1ft", "test t\nPACK p 1\nThing.twice getter: this.n * 2\ngiven Thing:a {\"n\": 2}\nreads Thing:a.twice = 4\neval Thing:a.twice + 1 = 5\n").unwrap();
    run_test(&ts[0], Path::new(".")).unwrap();
}

#[test]
fn a_bare_line_outside_a_test_and_a_bare_mistake_say_where() {
    for (text, want) in [("given Thing:a {}\n", "f.c1ft:1: a line outside a test"), ("test t\ngiven Thing:a not json\n", "f.c1ft:2: given: the properties are not JSON"), ("test t\nreads Thing:a = 1\n", "f.c1ft:2: reads:"), ("test \n", "f.c1ft:1: test: a name"), ("test t\ntest t\n", "f.c1ft:2: a second test named t")] {
        let e = parse("f.c1ft", text).unwrap_err();
        assert!(e.contains(want), "{text:?}: {e}");
    }
}

#[test]
fn a_bare_let_binds_a_variable_for_the_evals_below_it() {
    let ts = parse("t.c1ft", "test t\nlet xs = [1, 2]\nlet d = {'a': 3}\neval xs[0] + d['a'] = 4\n").unwrap();
    assert!(matches!(&ts[0].steps[0].step, Step::Let { name, expr } if name == "xs" && expr == "[1, 2]"));
    run_test(&ts[0], Path::new(".")).unwrap();
}
