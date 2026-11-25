//! Running one test: a fresh engine (no store, no effects), the loads, the test's own pack, then the steps in order. Every directive that changes the graph pumps afterwards,
//! as the seat does between rounds, so a read sees the settled graph.
use crate::format::{Feed, Line, Load, Step, Test, Want};
use cliffvm::fields::LinkVal;
use cliffvm::host::{Host, Options};
use serde_json::Value as J;
use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

thread_local! { static NOW: Cell<u64> = Cell::new(0); static STEP: Cell<u64> = Cell::new(0); }
/// the fake clock of `clock step <ms>`: nanoseconds, advancing by the step at every read
fn fake() -> u64 { NOW.with(|n| { let v = n.get(); n.set(v + STEP.with(|s| s.get())); v }) }

/// What a test sets on the process (the step budget, the clock, the counters) is put back when it ends, whatever way it ends.
/// (The counters are the engine's, one table per process: only a test that turned them on turns them off.)
struct Reset(Cell<bool>);
impl Drop for Reset {
    fn drop(&mut self) { cliffvm::expr::set_eval_budget(None); cliffvm::expr::set_clock(None); if self.0.get() { cliffvm::stats::set_on(false); } }
}

/// The folder of a `store` test's own store file: removed when the test ends.
struct StoreDir(PathBuf);
impl Drop for StoreDir { fn drop(&mut self) { let _ = std::fs::remove_dir_all(&self.0); } }
fn store_dir() -> Result<StoreDir, String> {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!("c1ft-store-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    std::fs::create_dir_all(&d).map_err(|e| format!("the store folder {}: {e}", d.display()))?;
    Ok(StoreDir(d))
}

enum Built { Ready(Host, HashMap<usize, String>), Done(Result<(), String>) }

/// The engine of a test, as the test's text makes it: open (on the store, if there is one), the namespace, the loads, the test's own pack, the pack of every `eval`, rebuilt.
/// `Done` is a test that ends here: the pack it expects to be refused.
fn build(t: &Test, base: &Path, store: Option<&Path>, reopen: bool) -> Built {
    macro_rules! fail { ($($a:tt)*) => { return Built::Done(Err(format!($($a)*))) } }
    let mut o = Options::new(store.map(|p| p.to_string_lossy().to_string()));
    o.effects = false;
    let h = match Host::open(o) { Ok(h) => h, Err(e) => fail!("{}: the engine did not open: {e}", at(t, t.line)) };
    if let Some(ns) = &t.ns { h.set_ns(ns); }
    for l in &t.loads {
        match l {
            Load::File(p) => match h.load_app(&base.join(p), false) {
                Ok(l) => if let Err(e) = refused(t, p, &l) { return Built::Done(Err(e)); },
                Err(e) => fail!("{}: load {p}: {e}", at(t, t.line)),
            },
            Load::List { file, dir } => {
                let text = match std::fs::read_to_string(base.join(file)) { Ok(x) => x, Err(e) => fail!("{}: list {file}: {e}", at(t, t.line)) };
                for (i, name) in list_names(&text).into_iter().enumerate() {
                    let p = format!("{dir}/{name}.c1f");
                    match h.load_app(&base.join(&p), i == 0) {
                        Ok(l) => if let Err(e) = refused(t, &p, &l) { return Built::Done(Err(e)); },
                        Err(e) => fail!("{}: list {file}: {p}: {e}", at(t, t.line)),
                    }
                }
            }
        }
    }
    if let Some(name) = t.pack.lines().find_map(|l| l.strip_prefix("PACK ")).and_then(|r| r.split_whitespace().next()) {
        let res = h.load_pack(name, &t.pack);
        match (&t.error, res) {
            (Some(want), Err(e)) => return Built::Done(if e.contains(want.as_str()) { Ok(()) } else { Err(format!("{}: the loader refused the pack with {e:?}, which does not hold {want:?}", at(t, t.line))) }),
            (Some(want), Ok(_)) => fail!("{}: the loader accepted a pack the test says is refused ({want:?})", at(t, t.line)),
            (None, Err(e)) => fail!("{}: the loader refused the pack: {e}", at(t, t.line)),
            (None, Ok(_)) => {}
        }
    } else if let Some(want) = &t.error {
        fail!("{}: the test expects a refusal ({want:?}) and has no pack", at(t, t.line));
    }
    // every `eval` is a getter of a pack of its own, loaded before the steps run (a refusal is kept for the step that asked)
    let mut refused_eval: HashMap<usize, String> = Default::default();
    let mut lets: Vec<String> = vec![];
    for (i, Line { step, .. }) in t.steps.iter().enumerate() {
        if let Step::Let { name, expr } = step { lets.push(format!("  {name} = {expr}\n")); }
        if let Step::Eval { expr, .. } = step {
            let pack = format!("PACK ceval{i} 1\nCeval{i}.val getter:\n{}  {expr}\nCeval{i}.shown getter: {{'v': this.val, 'why': why(this.val), 'text': str(this.val)}}\n", lets.concat());
            if let Err(e) = h.load_pack(&format!("ceval{i}"), &pack) { refused_eval.insert(i, e); }
        }
    }
    h.rebuild();
    if reopen { h.load_stored_entities(); h.pump(); }
    Built::Ready(h, refused_eval)
}

/// Ok when every step held; Err is one message naming the file and line.
pub fn run_test(t: &Test, base: &Path) -> Result<(), String> {
    let reset = Reset(Cell::new(false));
    let dir = if t.store { Some(store_dir()?) } else { None };
    let store_file = dir.as_ref().map(|d| d.0.join("s.sqlite"));
    let (h, mut refused_eval) = match build(t, base, store_file.as_deref(), false) { Built::Ready(h, r) => (h, r), Built::Done(r) => return r };
    let mut h = Some(h);
    let mut rev = 0.0f64;
    for (i, Line { line, step }) in t.steps.iter().enumerate() {
        let here = at(t, *line);
        let host = h.as_ref().expect("an engine");
        match step {
            Step::Given { eid, props } => { host.born(eid, class_of(eid), props.as_object().cloned().unwrap_or_default(), "person"); host.pump(); }
            Step::Write { eid, prop, value } => { host.write(eid, prop, value.clone(), "person"); host.pump(); }
            Step::Pump => host.pump(),
            Step::Reads { eid, prop, want } => {
                let got = host.get(eid, prop);
                if !same(&got, want) { return Err(format!("{here}: {eid}.{prop} reads {got}, expected {want}")); }
            }
            Step::Eval { expr, want } => {
                if let Some(e) = refused_eval.get(&i) {
                    match want { Want::Error(text) if e.contains(text.as_str()) => continue, Want::Error(text) => return Err(format!("{here}: eval {expr}: the loader refused it with {e:?}, which does not hold {text:?}")), Want::Value(_) => return Err(format!("{here}: eval {expr}: the loader refused the expression: {e}")) }
                }
                let eid = format!("Ceval{i}:x");
                host.born(&eid, &format!("Ceval{i}"), Default::default(), "person");
                host.pump();
                let shown = host.get(&eid, "shown");
                let (v, why, text) = (&shown["v"], shown["why"].as_str().unwrap_or(""), shown["text"].as_str().unwrap_or(""));
                // an evaluation the engine stopped (the step budget) keeps a null and says why in the entity's `failed`
                let failed = match host.get(&eid, "failed") { J::Null => String::new(), f => f.to_string() };
                match want {
                    Want::Value(w) => if !same(v, w) { return Err(format!("{here}: {expr} is {v}, expected {w}{}", if v.is_null() && !why.is_empty() { format!(" (why: {why})") } else if v.is_null() && !failed.is_empty() { format!(" (failed: {failed})") } else { String::new() })); },
                    Want::Error(w) => if !(why.contains(w.as_str()) || text.contains(w.as_str()) || failed.contains(w.as_str())) {
                        return Err(if v.is_null() { format!("{here}: {expr}: expected an error holding {w:?}; the reason is {why:?}, it prints {text:?} and it failed with {failed:?}") } else { format!("{here}: {expr} is {v}, expected an error holding {w:?}") });
                    },
                }
            }
            Step::Let { .. } => {}
            Step::Exists(eid) => if !host.exists(eid) { return Err(format!("{here}: {eid} does not exist")); },
            Step::Absent(eid) => if host.exists(eid) { return Err(format!("{here}: {eid} exists")); },
            Step::Type { expr, vars, want } => {
                let vs: Vec<(&str, &str)> = vars.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
                let (ty, _) = cliffvm::infer::infer_src(expr, &vs).map_err(|e| format!("{here}: {expr}: {e}"))?;
                if &ty != want { return Err(format!("{here}: {expr} has the type {ty}, expected {want}")); }
            }
            Step::Findings { expr, vars, want } => {
                let vs: Vec<(&str, &str)> = vars.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
                let (_, found) = cliffvm::infer::infer_src(expr, &vs).map_err(|e| format!("{here}: {expr}: {e}"))?;
                if found.len() != *want { return Err(format!("{here}: {expr} has {} finding(s) {found:?}, expected {want}", found.len())); }
            }
            Step::Budget(n) => cliffvm::expr::set_eval_budget(*n),
            Step::Clock(ms) => match ms {
                Some(ms) => { NOW.with(|n| n.set(0)); STEP.with(|s| s.set(ms * 1_000_000)); cliffvm::expr::set_clock(Some(fake)); }
                None => cliffvm::expr::set_clock(None),
            },
            Step::StatsOn(on) => { if *on { cliffvm::stats::global_clear(); reset.0.set(true); } cliffvm::stats::set_on(*on); }
            Step::Stats { kind, cls, prop, field, op, want } => {
                let kind_name = if kind == "getter" { cliffvm::stats::KIND_GETTER } else { cliffvm::stats::KIND_SETTER };
                let Some(r) = cliffvm::stats::global_lifetime().into_iter().find(|r| r.kind == kind_name && &r.cls == cls && &r.prop == prop) else { return Err(format!("{here}: stats: no row for the {kind} {cls}.{prop} (are the stats on?)")) };
                let got = match field.as_str() { "calls" => r.calls, "ns" => r.ns, "wait_ns" => r.wait_ns, "steps" => r.steps, f => return Err(format!("{here}: stats: no field {f} (calls, ns, wait_ns, steps)")) };
                let ok = match op.as_str() { "=" => got == *want, ">=" => got >= *want, _ => got <= *want };
                if !ok { return Err(format!("{here}: the {kind} {cls}.{prop} has {field} {got}, expected {op} {want}")); }
            }
            Step::Feed { eid, what } => {
                rev += 1.0;
                host.feed_val(eid, match what { Feed::Props(p) => LinkVal::Props(p.clone()), Feed::Gone => LinkVal::Gone, Feed::Refused(w) => LinkVal::Refused(w.clone()) }, rev);
                host.pump();
            }
            Step::Restart => {
                drop(h.take());
                match build(t, base, store_file.as_deref(), store_file.is_some()) { Built::Ready(nh, r) => { refused_eval = r; h = Some(nh); } Built::Done(r) => return r }
            }
        }
    }
    Ok(())
}
