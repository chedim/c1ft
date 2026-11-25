//! Running one test: a fresh engine (no store, no effects), the loads, the test's own pack, then the steps in order. Every directive that changes the graph pumps afterwards,
//! as the seat does between rounds, so a read sees the settled graph.
use crate::format::{Line, Load, Step, Test};
use cliffvm::host::{Host, Options};
use serde_json::Value as J;
use std::path::Path;

/// `ns::Class:key` is a `Class` (the class after a leading namespace), `Class:key` too
pub fn class_of(eid: &str) -> &str {
    let r = eid.split_once("::").map(|(_, r)| r).unwrap_or(eid);
    r.split(':').next().unwrap_or(r)
}
fn same(a: &J, b: &J) -> bool { match (a.as_f64(), b.as_f64()) { (Some(x), Some(y)) => (x - y).abs() < 1e-9, _ => a == b } }

/// The pack names of a list file: one per line, blank lines and `#` lines skipped.
pub fn list_names(text: &str) -> Vec<&str> { text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect() }

/// what the loader said about the packs of a load: a refused pack is the test's failure, never a silent pass
fn refused(t: &Test, what: &str, l: &cliffvm::host::Loaded) -> Result<(), String> {
    if l.msgs.is_empty() { Ok(()) } else { Err(format!("{}: load {what}: {}", at(t, t.line), l.msgs.join("; "))) }
}
fn at(t: &Test, n: usize) -> String { format!("{}:{}", t.file, n) }

/// Ok when every step held; Err is one message naming the file and line.
pub fn run_test(t: &Test, base: &Path) -> Result<(), String> {
    let mut o = Options::new(None);
    o.effects = false;
    let h = Host::open(o).map_err(|e| format!("{}: the engine did not open: {e}", at(t, t.line)))?;
    for l in &t.loads {
        match l {
            Load::File(p) => { let l = h.load_app(&base.join(p), false).map_err(|e| format!("{}: load {p}: {e}", at(t, t.line)))?; refused(t, p, &l)?; }
            Load::List { file, dir } => {
                let text = std::fs::read_to_string(base.join(file)).map_err(|e| format!("{}: list {file}: {e}", at(t, t.line)))?;
                for (i, name) in list_names(&text).into_iter().enumerate() {
                    let p = format!("{dir}/{name}.c1f");
                    let l = h.load_app(&base.join(&p), i == 0).map_err(|e| format!("{}: list {file}: {p}: {e}", at(t, t.line)))?;
                    refused(t, &p, &l)?;
                }
            }
        }
    }
    if let Some(name) = t.pack.lines().find_map(|l| l.strip_prefix("PACK ")).and_then(|r| r.split_whitespace().next()) {
        let res = h.load_pack(name, &t.pack);
        match (&t.error, res) {
            (Some(want), Err(e)) => { return if e.contains(want.as_str()) { Ok(()) } else { Err(format!("{}: the loader refused the pack with {e:?}, which does not hold {want:?}", at(t, t.line))) }; }
            (Some(want), Ok(_)) => return Err(format!("{}: the loader accepted a pack the test says is refused ({want:?})", at(t, t.line))),
            (None, Err(e)) => return Err(format!("{}: the loader refused the pack: {e}", at(t, t.line))),
            (None, Ok(_)) => {}
        }
    } else if let Some(want) = &t.error {
        return Err(format!("{}: the test expects a refusal ({want:?}) and has no pack", at(t, t.line)));
    }
    h.rebuild();
    for Line { line, step } in &t.steps {
        let here = at(t, *line);
        match step {
            Step::Given { eid, props } => { h.born(eid, class_of(eid), props.as_object().cloned().unwrap_or_default(), "person"); h.pump(); }
            Step::Write { eid, prop, value } => { h.write(eid, prop, value.clone(), "person"); h.pump(); }
            Step::Pump => h.pump(),
            Step::Reads { eid, prop, want } => {
                let got = h.get(eid, prop);
                if !same(&got, want) { return Err(format!("{here}: {eid}.{prop} reads {got}, expected {want}")); }
            }
            Step::Exists(eid) => if !h.exists(eid) { return Err(format!("{here}: {eid} does not exist")); },
            Step::Absent(eid) => if h.exists(eid) { return Err(format!("{here}: {eid} exists")); },
        }
    }
    Ok(())
}
