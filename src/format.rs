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
    /// `type <expr> [with <name>: <Type>, ...] = "<type>"`: the static inference's type of the expression
    Type { expr: String, vars: Vec<(String, String)>, want: String },
    /// `findings <expr> [with ...] = <n>`: how many problems the static check finds in the expression
    Findings { expr: String, vars: Vec<(String, String)>, want: usize },
    /// `budget <n>` / `budget off`: the step budget of the evaluations from here on
    Budget(Option<u64>),
    /// `clock step <ms>` / `clock off`: a fake clock that advances `ms` at every read, from here on
    Clock(Option<u64>),
    /// `stats on` / `stats off`: count (and clear) the engine's evaluation counters
    StatsOn(bool),
    /// `stats <getter|setter> <Class>.<prop> <field> (=|>=|<=) <n>`: a field of a counter row
    Stats { kind: String, cls: String, prop: String, field: String, op: String, want: u64 },
    /// `feed <eid> = {json}` / `feed gone <eid>` / `feed refused <eid> = "<why>"`: what a peer says of a node of its namespace
    Feed { eid: String, what: Feed },
    /// `restart`: close the engine and open it again on the same store (no store: on nothing)
    Restart,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Feed { Props(serde_json::Map<String, J>), Gone, Refused(String) }

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
    /// `store`: the engine runs on a store file of the test's own, so `restart` can reopen it
    pub store: bool,
    /// `ns <name>`: the engine's own namespace (a page's, say); ids under another namespace are foreign
    pub ns: Option<String>,
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

/// `a: Int, b: List(Int) | Error`: the declared types of an expression's variables; a comma inside parentheses belongs to the type
fn vars(file: &str, n: usize, word: &str, s: &str) -> Result<Vec<(String, String)>, String> {
    let (mut out, mut depth, mut cur) = (vec![], 0i32, String::new());
    for c in s.chars() {
        match c { '(' => depth += 1, ')' => depth -= 1, _ => {} }
        if c == ',' && depth == 0 { out.push(std::mem::take(&mut cur)); } else { cur.push(c); }
    }
    if !cur.trim().is_empty() { out.push(cur); }
    out.into_iter().map(|p| {
        let (name, ty) = p.split_once(':').ok_or_else(|| format!("{file}:{n}: {word}: `with <name>: <Type>, ...`"))?;
        let name = name.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || ty.trim().is_empty() { return Err(format!("{file}:{n}: {word}: `with <name>: <Type>, ...`")); }
        Ok((name.to_string(), ty.trim().to_string()))
    }).collect()
}
/// `<expr> [with <vars>]`
fn expr_with(file: &str, n: usize, word: &str, left: &str) -> Result<(String, Vec<(String, String)>), String> {
    match left.rsplit_once(" with ") { Some((e, v)) if v.contains(':') => Ok((e.trim().to_string(), vars(file, n, word, v)?)), _ => Ok((left.trim().to_string(), vec![])) }
}

/// The words that make a line a directive. A line is a directive when it starts with one at column 0, or with `-- ` and then one: `given Thing:a {}` and `-- given Thing:a {}` are the same.
const KEYWORDS: [&str; 21] = ["test", "load", "list", "given", "write", "pump", "reads", "eval", "let", "exists", "absent", "error:", "type", "findings", "budget", "clock", "stats", "feed", "ns", "store", "restart"];

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
                out.push(Test { file: file.into(), name: name.into(), line: n, pack: String::new(), loads: vec![], steps: vec![], error: None, store: false, ns: None, text: format!("{line}\n") });
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
            "type" => {
                let bad = || format!("{file}:{n}: type: `<expr> [with <name>: <Type>, ...] = \"<type>\"`");
                let (left, v) = rest.split_once(" = ").ok_or_else(bad)?;
                let want: String = serde_json::from_str(v.trim()).map_err(|_| bad())?;
                let (expr, vars) = expr_with(file, n, "type", left)?;
                if expr.is_empty() { return Err(bad()); }
                step(Step::Type { expr, vars, want });
            }
            "findings" => {
                let bad = || format!("{file}:{n}: findings: `<expr> [with <name>: <Type>, ...] = <n>`");
                let (left, v) = rest.split_once(" = ").ok_or_else(bad)?;
                let want: usize = v.trim().parse().map_err(|_| format!("{file}:{n}: findings: the count is not a whole number"))?;
                let (expr, vars) = expr_with(file, n, "findings", left)?;
                if expr.is_empty() { return Err(bad()); }
                step(Step::Findings { expr, vars, want });
            }
            "budget" => match rest.trim() { "off" => step(Step::Budget(None)), x => step(Step::Budget(Some(x.parse().map_err(|_| format!("{file}:{n}: budget: `<n>` or `off`"))?))) },
            "clock" => match rest.trim().split_whitespace().collect::<Vec<_>>()[..] {
                ["off"] => step(Step::Clock(None)),
                ["step", ms] => step(Step::Clock(Some(ms.parse().map_err(|_| format!("{file}:{n}: clock: `step <ms>` or `off`"))?))),
                _ => return Err(format!("{file}:{n}: clock: `step <ms>` or `off`")),
            },
            "stats" => match rest.trim().split_whitespace().collect::<Vec<_>>()[..] {
                ["on"] => step(Step::StatsOn(true)),
                ["off"] => step(Step::StatsOn(false)),
                [kind @ ("getter" | "setter"), target, field, op @ ("=" | ">=" | "<="), num] => {
                    let (cls, prop) = target.rsplit_once('.').ok_or_else(|| format!("{file}:{n}: stats: `<getter|setter> <Class>.<prop> <field> (=|>=|<=) <n>`"))?;
                    let want = num.parse().map_err(|_| format!("{file}:{n}: stats: the count is not a whole number"))?;
                    step(Step::Stats { kind: kind.into(), cls: cls.into(), prop: prop.into(), field: field.into(), op: op.into(), want });
                }
                [_, ..] if rest.trim().starts_with("getter") || rest.trim().starts_with("setter") => return Err(format!("{file}:{n}: stats: `<getter|setter> <Class>.<prop> <field> (=|>=|<=) <n>`")),
                _ => return Err(format!("{file}:{n}: stats: `on`, `off` or `<getter|setter> <Class>.<prop> <field> (=|>=|<=) <n>`")),
            },
            "feed" => {
                if let Some(eid) = rest.trim().strip_prefix("gone ") { step(Step::Feed { eid: eid.trim().into(), what: Feed::Gone }); }
                else if let Some(r) = rest.trim().strip_prefix("refused ") {
                    let (eid, why) = r.split_once(" = ").ok_or_else(|| format!("{file}:{n}: feed: `refused <eid> = \"<why>\"`"))?;
                    let why: String = serde_json::from_str(why.trim()).map_err(|_| format!("{file}:{n}: feed: `refused <eid> = \"<why>\"`"))?;
                    step(Step::Feed { eid: eid.trim().into(), what: Feed::Refused(why) });
                } else {
                    let (eid, props) = rest.split_once(" = ").ok_or_else(|| format!("{file}:{n}: feed: `<eid> = {{json}}`, `gone <eid>` or `refused <eid> = \"<why>\"`"))?;
                    let props = serde_json::from_str::<J>(props.trim()).ok().and_then(|j| j.as_object().cloned()).ok_or_else(|| format!("{file}:{n}: feed: the properties are not a JSON object"))?;
                    step(Step::Feed { eid: eid.trim().into(), what: Feed::Props(props) });
                }
            }
            "ns" => { if rest.trim().is_empty() { return Err(format!("{file}:{n}: ns: a name")); } t.ns = Some(rest.trim().into()); }
            "store" => t.store = true,
            "restart" => step(Step::Restart),
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
