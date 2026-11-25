//! The directives that look at the engine's own records: `replay` and `same`, `journal`, `privileged`, `row` and `refusals` (docs/format.md).
use c1ft::format::{parse, Step};
use c1ft::run::run_test;
use std::path::Path;

fn one(body: &str) -> Result<(), String> {
    let ts = parse("t.c1ft", &format!("test t\n{body}")).unwrap();
    run_test(&ts[0], Path::new("."))
}

const TWO: &str = "PACK two 1\nAlpha.sa getter: 'alpha:' + str(this.n)\nAlpha.go_a on-setter run-a:\n  WHEN this.go_a == True:\n    this.did_a = True\n";

#[test]
fn every_directive_parses_to_its_step_and_a_mistake_names_the_line() {
    let ts = parse("t.c1ft", "test t\nreplay\nsame Thing:x\nsame all\njournal only outside\njournal lacks write granted_by\njournal has born Thing:x\njournal has write Thing:x.n\nprivileged plain = true\nrow Box:a lacks label\nrow Box:a has w\nrefusals = 2\nrefusal 0 holds \"loop\"\n").unwrap();
    let k: Vec<&str> = ts[0].steps.iter().map(|s| match &s.step { Step::Replay => "replay", Step::Same(_) => "same", Step::Journal(_) => "journal", Step::Privileged { .. } => "privileged", Step::Row { .. } => "row", Step::Refusals(_) => "refusals", Step::Refusal { .. } => "refusal", _ => "other" }).collect();
    assert_eq!(k, ["replay", "same", "same", "journal", "journal", "journal", "journal", "privileged", "row", "row", "refusals", "refusal"]);
    for (text, want) in [
        ("same\n", "f.c1ft:2: same: `<eid>` or `all`"),
        ("journal sideways\n", "f.c1ft:2: journal: `only outside`, `lacks write <prop>`, `has born <eid>` or `has write <eid>.<prop>`"),
        ("privileged plain\n", "f.c1ft:2: privileged: `<pack> = true|false`"),
        ("privileged plain = maybe\n", "f.c1ft:2: privileged: `<pack> = true|false`"),
        ("row Box:a\n", "f.c1ft:2: row: `<eid> has <prop>` or `<eid> lacks <prop>`"),
        ("refusals = many\n", "f.c1ft:2: refusals: the count is not a whole number"),
        ("refusal x holds \"a\"\n", "f.c1ft:2: refusal: `<n> holds \"<text>\"`"),
    ] {
        let e = parse("f.c1ft", &format!("test t\n{text}")).unwrap_err();
        assert!(e.contains(want), "{text:?}: {e}");
    }
}

#[test]
fn a_replay_rebuilds_the_graph_from_the_journal_and_same_says_the_entity_is_what_it_was() {
    one(&format!("{TWO}given cls:alpha {{\"name\": \"Alpha\"}}\ngiven Thing:x {{\"n\": 3}}\nwrite Thing:x.parents = [\"cls:alpha\"]\nwrite Thing:x.n = 4\nwrite Thing:x.go_a = true\nreplay\nreads Thing:x.sa = \"alpha:4\"\nreads Thing:x.did_a = true\nsame Thing:x\nsame all\n")).unwrap();
}

#[test]
fn same_without_a_replay_and_a_replay_that_differs_fail_with_the_line() {
    let e = one("given Thing:x {\"n\": 1}\nsame Thing:x\n").unwrap_err();
    assert!(e.contains("t.c1ft:3") && e.contains("replay"), "{e}");
    // an entity the replay does not have
    let e = one("given Thing:x {\"n\": 1}\nreplay\nsame Thing:nope\n").unwrap_err();
    assert!(e.contains("t.c1ft:4") && e.contains("Thing:nope"), "{e}");
    // what the journal does not hold (a write the engine derived) differs: a given after the replay is not in the original
    let e = one("given Thing:x {\"n\": 1}\nreplay\ngiven Thing:y {\"n\": 2}\nsame all\n").unwrap_err();
    assert!(e.contains("t.c1ft:5") && e.contains("same"), "{e}");
}

#[test]
fn the_journal_holds_the_outside_writes_only_and_a_derived_write_is_not_in_it() {
    one("PACK t 1\nBox.go on-setter s:\n  ALWAYS:\n    this.derived = 1\ngiven Box:a {\"w\": 1}\nwrite Box:a.go = 1\nreads Box:a.derived = 1\njournal only outside\njournal has born Box:a\njournal has write Box:a.go\njournal lacks write derived\n").unwrap();
    let e = one("given Box:a {\"w\": 1}\njournal has write Box:a.w\n").unwrap_err();
    assert!(e.contains("t.c1ft:3") && e.contains("Box:a.w"), "{e}");
    let e = one("given Box:a {\"w\": 1}\nwrite Box:a.w = 2\njournal lacks write w\n").unwrap_err();
    assert!(e.contains("t.c1ft:4") && e.contains("w"), "{e}");
    let e = one("given Box:a {}\njournal has born Box:zzz\n").unwrap_err();
    assert!(e.contains("t.c1ft:3") && e.contains("Box:zzz"), "{e}");
}

#[test]
fn a_pack_is_privileged_when_it_has_been_granted_and_revoked_when_it_is_taken_back() {
    one("PACK plain 1\nTrig.go on-setter w:\n  ALWAYS: Cls:a.val = 1\nprivileged plain = false\ngiven Pack:plain {\"privileged\": true}\nprivileged plain = true\nwrite Pack:plain.privileged = false\nprivileged plain = false\n").unwrap();
    let e = one("PACK plain 1\nTrig.go on-setter w:\n  ALWAYS: Cls:a.val = 1\nprivileged plain = true\n").unwrap_err();
    assert!(e.contains("t.c1ft:5") && e.contains("plain") && e.contains("not privileged"), "{e}");
}

#[test]
fn a_stored_row_has_what_is_stored_and_lacks_what_is_derived() {
    one("PACK mine 1\nBox.label getter: 'l' + str(this.w)\ngiven Box:a {\"w\": 1}\nreads Box:a.label = \"l1\"\nrow Box:a has w\nrow Box:a lacks label\n").unwrap();
    let e = one("given Box:a {\"w\": 1}\nrow Box:a lacks w\n").unwrap_err();
    assert!(e.contains("t.c1ft:3") && e.contains("Box:a") && e.contains("w"), "{e}");
    let e = one("given Box:a {\"w\": 1}\nrow Box:a has nope\n").unwrap_err();
    assert!(e.contains("t.c1ft:3") && e.contains("nope"), "{e}");
    let e = one("row Box:zzz has w\n").unwrap_err();
    assert!(e.contains("Box:zzz"), "{e}");
}

#[test]
fn the_refusals_the_engine_keeps_can_be_counted_and_read() {
    let pack = "PACK t 1\nThing.shown getter: this.eid\nBox.go on-setter t:\n  ALWAYS:\n    Thing:t1 = Nope(kind: 'in')\n";
    one(&format!("{pack}refusals = 0\ngiven Box:a {{}}\nwrite Box:a.go = 1\nrefusals = 1\nrefusal 0 holds \"Nope\"\n")).unwrap();
    let e = one(&format!("{pack}given Box:a {{}}\nwrite Box:a.go = 1\nrefusals = 5\n")).unwrap_err();
    assert!(e.contains("t.c1ft:") && e.contains("1 refusal") && e.contains("expected 5"), "{e}");
    let e = one(&format!("{pack}given Box:a {{}}\nwrite Box:a.go = 1\nrefusal 0 holds \"never said\"\n")).unwrap_err();
    assert!(e.contains("never said") && e.contains("Nope"), "the failure shows what the refusal said: {e}");
    let e = one("refusal 3 holds \"x\"\n").unwrap_err();
    assert!(e.contains("no refusal 3"), "{e}");
}
