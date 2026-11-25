//! The file format (docs/format.md). A file is a list of tests; `-- test <name>` starts one. The lines of a test that are not directives are its pack (c1f text,
//! `--` comments and all); the directives are `load`, `list`, `given`, `write`, `pump`, `reads`, `eval`, `let`, `exists`, `absent` and `error:`.
use serde_json::Value as J;

#[derive(Debug, Clone, PartialEq)]
pub enum Load {
    /// `-- load <path>`: a pack file, or a folder of them (an app)
    File(String),
    /// `-- list <list file> <dir>`: the names in the list file, each `<dir>/<name>.c1f`, the first one trusted, as the seat's boot loads them
    List { file: String, dir: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Given { eid: String, props: J },
    Write { eid: String, prop: String, value: J },
    Pump,
    Reads { eid: String, prop: String, want: J },
    Exists(String),
    Absent(String),
    /// `-- eval <expr> = <json>` / `-- eval <expr> = error: <text>`: the expression is evaluated on the engine as the body of a getter
    Eval { expr: String, want: Want },
    /// `-- let <name> = <expr>`: a variable the evals below it see
    Let { name: String, expr: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Want {
    Value(J),
    /// the expression fails (the loader refuses it, or its value is an error, or a null with a reason) and the message holds the text
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line { pub line: usize, pub step: Step }

#[derive(Debug, Clone)]
pub struct Test {
    /// the file as it was named to the parser (what the messages say)
    pub file: String,
    pub name: String,
    /// the line of `-- test <name>`
    pub line: usize,
    /// the test's own pack text: its lines that are not directives
    pub pack: String,
    pub loads: Vec<Load>,
    pub steps: Vec<Line>,
    /// `-- error: <text>`: the pack must be refused and the loader's message hold the text
    pub error: Option<String>,
    /// the test's own lines, directives included: what its cache key hashes
    pub text: String,
}

fn json(file: &str, n: usize, what: &str, s: &str) -> Result<J, String> {
    serde_json::from_str(s.trim()).map_err(|e| format!("{file}:{n}: {what}: {e}"))
}
/// `<eid>.<prop> = <json>`: the last dot before the `=` splits the target
fn target(file: &str, n: usize, word: &str, rest: &str, with_value: bool) -> Result<(String, String, Option<J>), String> {
    let bad = || format!("{file}:{n}: {word}: `<eid>.<prop> = <json>`");
    let (t, v) = if with_value { let (t, v) = rest.split_once(" = ").ok_or_else(bad)?; (t, Some(v)) } else { (rest, None) };
    let (e, p) = t.trim().rsplit_once('.').ok_or_else(bad)?;
    if e.is_empty() || p.is_empty() { return Err(bad()); }
    let v = match v { Some(v) => Some(json(file, n, &format!("{word}: the value is not JSON"), v).map_err(|m| m.replace(": the value is not JSON: ", ": the value is not JSON ("))?), None => None };
    Ok((e.to_string(), p.to_string(), v))
}

/// The words that make a line a directive. A line is a directive when it starts with one at column 0, or with `-- ` and then one: `given Thing:a {}` and `-- given Thing:a {}` are the same.
const KEYWORDS: [&str; 12] = ["test", "load", "list", "given", "write", "pump", "reads", "eval", "let", "exists", "absent", "error:"];

/// `Some(<the line without its `-- `>)` when the line is a directive, None when it is the test's pack or a comment
fn directive(line: &str) -> Option<&str> {
    let d = line.strip_prefix("-- ").unwrap_or(line);
    let w = d.split_whitespace().next().unwrap_or("");
    if KEYWORDS.contains(&w) || w.starts_with("error:") { Some(d) } else { None }
}

pub fn parse(file: &str, text: &str) -> Result<Vec<Test>, String> {
    let mut out: Vec<Test> = vec![];
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let line = raw.trim_end();
        if let Some(rest) = directive(line).and_then(|d| d.strip_prefix("test")) {
            if rest.is_empty() || rest.starts_with(' ') {
                let name = rest.trim();
                if name.is_empty() { return Err(format!("{file}:{n}: test: a name")); }
                if out.iter().any(|t| t.name == name) { return Err(format!("{file}:{n}: a second test named {name}")); }
                out.push(Test { file: file.into(), name: name.into(), line: n, pack: String::new(), loads: vec![], steps: vec![], error: None, text: format!("{line}\n") });
                continue;
            }
        }
        let Some(t) = out.last_mut() else {
            if line.trim().is_empty() || line.starts_with("--") { continue; }
            return Err(format!("{file}:{n}: a line outside a test: {line}"));
        };
        t.text.push_str(raw); t.text.push('\n');
        let Some(d) = directive(line) else { t.pack.push_str(raw); t.pack.push('\n'); continue };
        let (word, rest) = d.split_once(' ').map(|(w, r)| (w, r)).unwrap_or((d, ""));
        let mut step = |s: Step| t.steps.push(Line { line: n, step: s });
        match word {
            "load" => { if rest.trim().is_empty() { return Err(format!("{file}:{n}: load: a path")); } t.loads.push(Load::File(rest.trim().into())); }
            "list" => {
                let mut p = rest.split_whitespace();
                match (p.next(), p.next(), p.next()) { (Some(f), Some(d), None) => t.loads.push(Load::List { file: f.into(), dir: d.into() }), _ => return Err(format!("{file}:{n}: list: `<list file> <dir>`")) }
            }
            "given" => {
                let (eid, props) = rest.split_once(' ').ok_or_else(|| format!("{file}:{n}: given: `<eid> <json properties>`"))?;
                let props = serde_json::from_str::<J>(props.trim()).ok().filter(J::is_object).ok_or_else(|| format!("{file}:{n}: given: the properties are not JSON (an object)"))?;
                step(Step::Given { eid: eid.into(), props });
            }
            "write" => { let (eid, prop, v) = target(file, n, "write", rest, true)?; step(Step::Write { eid, prop, value: v.unwrap() }); }
            "pump" => step(Step::Pump),
            "reads" => { let (eid, prop, v) = target(file, n, "reads", rest, true)?; step(Step::Reads { eid, prop, want: v.unwrap() }); }
            "eval" => {
                let bad = || format!("{file}:{n}: eval: `<expression> = <json>` (or `= error: <text>`)");
                let (expr, v) = rest.split_once(" = ").ok_or_else(bad)?;
                if expr.trim().is_empty() { return Err(bad()); }
                let want = match v.trim().strip_prefix("error:") { Some(t) => Want::Error(t.trim().into()), None => Want::Value(json(file, n, "eval: the value is not JSON", v)?) };
                step(Step::Eval { expr: expr.trim().into(), want });
            }
            "let" => {
                let bad = || format!("{file}:{n}: let: `<name> = <expression>`");
                let (name, expr) = rest.split_once(" = ").ok_or_else(bad)?;
                let name = name.trim();
                if name.is_empty() || expr.trim().is_empty() { return Err(bad()); }
                if !name.chars().next().map_or(false, |c| c.is_ascii_alphabetic() || c == '_') || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') { return Err(format!("{file}:{n}: let: `{name}` is not a name")); }
                if let Some(Line { line: first, .. }) = t.steps.iter().find(|l| matches!(&l.step, Step::Let { name: m, .. } if m == name)) { return Err(format!("{file}:{n}: let: `{name}` is already a variable of this test (line {first}); a name is let once, as a local is in c1f")); }
                t.steps.push(Line { line: n, step: Step::Let { name: name.into(), expr: expr.trim().into() } });
            }
            "exists" => step(Step::Exists(rest.trim().into())),
            "absent" => step(Step::Absent(rest.trim().into())),
            _ => {
                if let Some(e) = d.strip_prefix("error:") { t.error = Some(e.trim().into()); }
                else { t.pack.push_str(raw); t.pack.push('\n'); }   // any other `-- ` line is a comment of the pack
            }
        }
    }
    Ok(out)
}
