//! The directives that say something about the engine itself: `type`, `findings`, `budget`, `clock`, `stats`, `ns`, `feed`, `store` and `restart` (docs/format.md).
use c1ft::format::{parse, Step};
use c1ft::run::run_test;
use std::path::Path;
use std::sync::Mutex;

static STATS: Mutex<()> = Mutex::new(());   // the evaluation counters are the engine's, one table per process

fn one(body: &str) -> Result<(), String> {
    let ts = parse("t.c1ft", &format!("test t\n{body}")).unwrap();
    run_test(&ts[0], Path::new("."))
}

#[test]
fn every_directive_parses_to_its_step_and_a_mistake_names_the_line() {
    let ts = parse("t.c1ft", "test t\nns p1\nstore\ntype Int(x) with x: Text, y: List(Int) | Error = \"Int|Error\"\nfindings 1 + 'a' = 1\nbudget 8\nbudget off\nclock step 6\nclock off\nstats on\nstats getter A.dbl calls >= 25\nstats off\nfeed Thing:a = {\"n\": 1}\nfeed gone Thing:a\nfeed refused Thing:a = \"too large\"\nrestart\n").unwrap();
    let t = &ts[0];
    assert_eq!(t.ns.as_deref(), Some("p1"));
    assert!(t.store);
    assert!(matches!(&t.steps[0].step, Step::Type { expr, vars, want } if expr == "Int(x)" && vars == &vec![("x".to_string(), "Text".to_string()), ("y".to_string(), "List(Int) | Error".to_string())] && want == "Int|Error"));
    assert!(matches!(&t.steps[1].step, Step::Findings { expr, vars, want } if expr == "1 + 'a'" && vars.is_empty() && *want == 1));
    assert!(matches!(&t.steps[2].step, Step::Budget(Some(8))) && matches!(&t.steps[3].step, Step::Budget(None)));
    assert!(matches!(&t.steps[4].step, Step::Clock(Some(6))) && matches!(&t.steps[5].step, Step::Clock(None)));
    assert!(matches!(&t.steps[6].step, Step::StatsOn(true)) && matches!(&t.steps[8].step, Step::StatsOn(false)));
    assert!(matches!(&t.steps[7].step, Step::Stats { kind, cls, prop, field, op, want } if kind == "getter" && cls == "A" && prop == "dbl" && field == "calls" && op == ">=" && *want == 25));
    assert_eq!(t.steps.len(), 13);
    assert!(matches!(&t.steps[12].step, Step::Restart));
    for (text, want) in [
        ("type Int(x) = 3\n", "f.c1ft:2: type: `<expr> [with <name>: <Type>, ...] = \"<type>\"`"),
        ("findings 1 = many\n", "f.c1ft:2: findings: the count is not a whole number"),
        ("budget soon\n", "f.c1ft:2: budget: `<n>` or `off`"),
        ("clock 5\n", "f.c1ft:2: clock: `step <ms>` or `off`"),
        ("stats getter A.dbl calls 5\n", "f.c1ft:2: stats: `<getter|setter> <Class>.<prop> <field> (=|>=|<=) <n>`"),
        ("stats loud\n", "f.c1ft:2: stats: `on`, `off` or `<getter|setter> <Class>.<prop> <field> (=|>=|<=) <n>`"),
        ("feed Thing:a = 5\n", "f.c1ft:2: feed: the properties are not a JSON object"),
        ("feed refused Thing:a\n", "f.c1ft:2: feed: `refused <eid> = \"<why>\"`"),
        ("ns\n", "f.c1ft:2: ns: a name"),
    ] {
        let e = parse("f.c1ft", &format!("test t\n{text}")).unwrap_err();
        assert!(e.contains(want), "{text:?}: {e}");
    }
}

// ---- type, findings -----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn type_says_what_the_inference_makes_of_an_expression_and_findings_how_many_problems_it_finds() {
    one("type Error('x') = \"Error\"\ntype Int(x) with x: Text = \"Int|Error\"\ntype Int(x) with x: Int = \"Int\"\ntype Int(x) with x: Text? = \"(Int|Error)?\"\ntype Voice(kind: 'in') = \"Entity(Voice)\"\nfindings Voice(kind: 1 + 'a') = 1\nfindings 1 + 1 = 0\n").unwrap();
    let e = one("type 1 + 1 = \"Text\"\n").unwrap_err();
    assert!(e.contains("t.c1ft:2") && e.contains("1 + 1") && e.contains("Int") && e.contains("Text"), "{e}");
    let e = one("findings 1 + 'a' = 0\n").unwrap_err();
    assert!(e.contains("t.c1ft:2") && e.contains("1 finding") && e.contains("expected 0"), "{e}");
    let e = one("type 1 + = \"Int\"\n").unwrap_err();
    assert!(e.contains("t.c1ft:2"), "an expression that does not parse says so: {e}");
}

// ---- budget, clock ------------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn a_budget_limits_the_steps_of_the_evals_below_it_until_it_is_lifted() {
    one("budget 3\neval 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 = error: budget\nbudget off\neval 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 = 36\n").unwrap();
    let e = one("budget 1000\neval 1 + 2 = 4\n").unwrap_err();
    assert!(e.contains("t.c1ft:3"), "{e}");
}

#[test]
fn a_budget_and_a_clock_do_not_leak_into_the_next_test() {
    one("budget 3\nclock step 5\n").unwrap();
    one("eval 1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 = 36\n").unwrap();
}

#[test]
fn a_clock_that_steps_makes_the_measured_time_of_an_evaluation_what_the_test_says() {
    let _g = STATS.lock().unwrap_or_else(|e| e.into_inner());
    one("PACK t 1\nBox.fine getter: this.w + 1\nstats on\nclock step 5\ngiven Box:a {\"w\": 1}\nstats getter Box.fine calls = 1\nstats getter Box.fine ns >= 5000000\nstats off\n").unwrap();
}

// ---- stats --------------------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn stats_count_the_evaluations_of_a_getter_and_a_setter() {
    let _g = STATS.lock().unwrap_or_else(|e| e.into_inner());
    one("PACK sa 1\nStatsA.dbl getter: this.w * 2\nStatsA.w on-setter copy:\n  ALWAYS:\n    this.z = this.w + 1\nstats on\ngiven StatsA:a {\"w\": 1}\nwrite StatsA:a.w = 10\nwrite StatsA:a.w = 11\nwrite StatsA:a.w = 12\nstats getter StatsA.dbl calls >= 3\nstats getter StatsA.dbl steps >= 9\nstats setter StatsA.w calls = 3\nstats off\nreads StatsA:a.z = 13\n").unwrap();
    let e = one("PACK sa 1\nStatsA.dbl getter: this.w * 2\nstats on\ngiven StatsA:a {\"w\": 1}\nstats getter StatsA.dbl calls = 99\n").unwrap_err();
    assert!(e.contains("t.c1ft:") && e.contains("StatsA.dbl") && e.contains("calls") && e.contains("99"), "{e}");
    let e = one("PACK sa 1\nStatsA.dbl getter: this.w * 2\nstats on\ngiven StatsA:a {\"w\": 1}\nstats getter Nope.x calls = 1\n").unwrap_err();
    assert!(e.contains("Nope.x") && e.contains("no row"), "{e}");
}

// ---- ns, feed -----------------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn a_foreign_read_is_pending_then_absent_then_refused_as_classes() {
    one("ns p1\nPACK t 1\nLight.c getter: why(membrane::Setting:t.val).cls\nLight.t getter: why(membrane::Setting:t.val).text\ngiven Light:a {\"x\": 1}\nreads Light:a.c = \"Pending\"\nreads Light:a.t = \"pending: membrane::Setting:t.val\"\nfeed gone membrane::Setting:t\nreads Light:a.c = \"Absent\"\nfeed refused membrane::Setting:t = \"too large (70000 bytes)\"\nreads Light:a.c = \"Refused\"\nreads Light:a.t = \"refused: too large (70000 bytes)\"\n").unwrap();
}

#[test]
fn a_fed_value_is_read_like_any_other_and_follows_the_next_feed() {
    one("ns p1\nPACK t 1\nLight.v getter: membrane::Setting:t.val\ngiven Light:a {}\nfeed membrane::Setting:t = {\"val\": 4}\nreads Light:a.v = 4\nfeed membrane::Setting:t = {\"val\": 9}\nreads Light:a.v = 9\n").unwrap();
}

// ---- store, restart -----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn a_restart_reopens_the_same_store_and_what_was_written_is_still_there() {
    one("store\nPACK t 1\nNote.twice getter: this.n * 2\ngiven Note:a {\"n\": 1}\nwrite Note:a.n = 2\nrestart\nreads Note:a.n = 2\nreads Note:a.twice = 4\nwrite Note:a.n = 5\nreads Note:a.twice = 10\n").unwrap();
}

#[test]
fn without_a_store_a_restart_forgets_everything() {
    one("PACK t 1\nNote.twice getter: this.n * 2\ngiven Note:a {\"n\": 1}\nrestart\nabsent Note:a\n").unwrap();
}

#[test]
fn a_store_is_removed_when_the_test_ends() {
    let dir = std::env::temp_dir();
    let before = std::fs::read_dir(&dir).unwrap().filter(|e| e.as_ref().map_or(false, |e| e.file_name().to_string_lossy().starts_with("c1ft-store-"))).count();
    one("store\ngiven Note:a {\"n\": 1}\nrestart\nexists Note:a\n").unwrap();
    let after = std::fs::read_dir(&dir).unwrap().filter(|e| e.as_ref().map_or(false, |e| e.file_name().to_string_lossy().starts_with("c1ft-store-"))).count();
    assert!(after <= before, "a test left its store behind");
}
