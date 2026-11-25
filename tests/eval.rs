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
