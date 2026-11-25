//! `-- eval <expr> = <json>` and `-- eval <expr> = error: <text>`: an expression is evaluated on the engine (as the body of a getter) and its value compared, or its failure expected.
use c1ft::format::{parse, Step};
use c1ft::run::run_test;
use std::path::Path;

fn one(body: &str) -> Result<(), String> {
    let ts = parse("t.c1ft", &format!("-- test t\n{body}")).unwrap();
    run_test(&ts[0], Path::new("."))
}

#[test]
fn an_expression_is_parsed_as_an_eval_step_with_its_expectation() {
    let ts = parse("t.c1ft", "-- test t\n-- eval 1 + 2 = 3\n-- eval keys([1]) = error: keys\n").unwrap();
    assert!(matches!(&ts[0].steps[0].step, Step::Eval { expr, want } if expr == "1 + 2" && *want == c1ft::format::Want::Value(serde_json::json!(3))));
    assert!(matches!(&ts[0].steps[1].step, Step::Eval { expr, want } if expr == "keys([1])" && *want == c1ft::format::Want::Error("keys".into())));
    for (text, want) in [("-- eval 1 + 2\n", "f.c1ft:2: eval: `<expression> = <json>`"), ("-- eval  = 3\n", "f.c1ft:2: eval: `<expression> = <json>`"), ("-- eval 1 = not json\n", "f.c1ft:2: eval: the value is not JSON")] {
        let e = parse("f.c1ft", &format!("-- test t\n{text}")).unwrap_err();
        assert!(e.contains(want), "{text:?}: {e}");
    }
}

#[test]
fn an_expression_that_evaluates_to_the_value_passes_and_a_wrong_value_says_what_it_was() {
    one("-- eval 1 + 2 = 3\n-- eval [x * 2 for x in [1, 2]] = [2, 4]\n-- eval {'a': 1}['a'] = 1\n-- eval 4 == 4 = true\n").unwrap();
    let e = one("-- eval 1 + 2 = 4\n").unwrap_err();
    assert!(e.contains("t.c1ft:2") && e.contains("1 + 2") && e.contains("is 3") && e.contains("expected 4"), "{e}");
}

#[test]
fn an_expression_reads_the_entities_the_test_has_given_and_written_so_far() {
    one("-- given Command:x {\"status\": \"ok\", \"rc\": 0}\n-- eval ${'Command:x'}['status'] = \"ok\"\n-- write Command:x.rc = 7\n-- eval ${'Command:x'}['rc'] = 7\n-- eval Command:x.rc + 1 = 8\n").unwrap();
}

#[test]
fn a_test_may_use_its_own_pack_and_an_expression_may_call_its_getters() {
    one("PACK p 1\nThing.twice getter: this.n * 2\n-- given Thing:a {\"n\": 2}\n-- eval Thing:a.twice = 4\n").unwrap();
}

#[test]
fn an_expression_that_fails_is_expected_with_the_text_its_reason_holds() {
    one("-- eval keys([1, 2]) = error: keys\n-- eval ${'A:9'}['d'] = error: A:9\n").unwrap();
    let e = one("-- eval 1 + 2 = error: anything\n").unwrap_err();
    assert!(e.contains("t.c1ft:2") && e.contains("expected an error") && e.contains("anything") && e.contains("is 3"), "{e}");
    let e = one("-- eval keys([1, 2]) = error: a text the reason never holds\n").unwrap_err();
    assert!(e.contains("a text the reason never holds") && e.contains("keys"), "the failure shows the reason it did give: {e}");
}

#[test]
fn an_expression_the_loader_refuses_is_an_error_it_can_be_expected_to_hold() {
    let e = one("-- eval 1 + = 3\n").unwrap_err();
    assert!(e.contains("t.c1ft:2"), "{e}");
    one("-- eval $x = error: $\n").unwrap();
    let e = one("-- eval $x = 3\n").unwrap_err();
    assert!(e.contains("t.c1ft:2") && e.contains("refused"), "{e}");
}

#[test]
fn an_eval_wrong_value_in_a_file_fails_the_run_with_its_line() {
    let e = one("-- given Command:x {\"rc\": 0}\n-- eval Command:x.rc = 1\n").unwrap_err();
    assert!(e.contains("t.c1ft:3") && e.contains("Command:x.rc") && e.contains("is 0"), "{e}");
}

// ---- `-- let <name> = <expr>`: a variable for the evals below it ---------------------------------------------------------------------------------------------------

#[test]
fn a_let_is_parsed_with_its_name_and_expression() {
    let ts = parse("t.c1ft", "-- test t\n-- let xs = [10, 20, 30]\n-- eval xs[0] = 10\n").unwrap();
    assert!(matches!(&ts[0].steps[0].step, Step::Let { name, expr } if name == "xs" && expr == "[10, 20, 30]"));
    for (text, want) in [("-- let xs\n", "f.c1ft:2: let: `<name> = <expression>`"), ("-- let  = 1\n", "f.c1ft:2: let: `<name> = <expression>`"), ("-- let 9x = 1\n", "f.c1ft:2: let: `9x` is not a name"), ("-- let a b = 1\n", "f.c1ft:2: let: `a b` is not a name")] {
        let e = parse("f.c1ft", &format!("-- test t\n{text}")).unwrap_err();
        assert!(e.contains(want), "{text:?}: {e}");
    }
}

#[test]
fn a_variable_is_seen_by_the_evals_below_it_and_not_by_those_above() {
    one("-- let xs = [10, 20, 30]\n-- let d = {'a': 1, 'b': ['x']}\n-- eval xs[-1] = 30\n-- eval d['b'][0] = \"x\"\n-- eval len(xs) + d['a'] = 4\n").unwrap();
    let e = one("-- eval xs[0] = 10\n-- let xs = [10]\n").unwrap_err();
    assert!(e.contains("t.c1ft:2"), "an eval above the let does not see it: {e}");
}

#[test]
fn a_name_is_let_once_per_test_as_a_local_is_in_c1f_and_a_let_may_use_an_earlier_one() {
    one("-- let n = 1\n-- let m = n + 1\n-- eval m = 2\n").unwrap();
    let e = parse("f.c1ft", "-- test t\n-- let n = 1\n-- eval n = 1\n-- let n = 2\n").unwrap_err();
    assert!(e.contains("f.c1ft:4") && e.contains("`n`") && e.contains("line 2"), "{e}");
    parse("f.c1ft", "-- test t\n-- let n = 1\n\n-- test u\n-- let n = 2\n").unwrap();   // another test, another set of variables
}

#[test]
fn a_variable_may_hold_an_entity_a_null_or_what_a_given_made() {
    one("-- given Thing:a {\"n\": 5}\n-- let e = ${'Thing:a'}\n-- let none = None\n-- eval e['n'] = 5\n-- eval none = null\n-- eval none + 1 = error: absent\n").unwrap();
}

#[test]
fn a_let_that_fails_makes_the_evals_below_it_fail_with_its_reason() {
    one("-- let bad = keys([1])\n-- eval bad = error: keys\n").unwrap();
}
