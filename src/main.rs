//! `c1ft run | list | watch` (docs/runner.md).
use c1ft::{cache, format, run};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

const USAGE: &str = "usage: c1ft run [--cache-dir <dir>] [--no-cache] [--filter <text>] [<path>...]
       c1ft list [<path>...]
       c1ft watch --every-ms <n> [--rounds <n>] [--cache-dir <dir>] [--no-cache] [--filter <text>] [<path>...]
a <path> is a .c1ft file or a folder searched for them (default: the current folder)";

struct Opts { cmd: String, cache_dir: PathBuf, no_cache: bool, filter: Option<String>, every_ms: Option<u64>, rounds: Option<u64>, paths: Vec<PathBuf> }

fn usage(msg: &str) -> ! { eprintln!("c1ft: {msg}\n{USAGE}"); std::process::exit(2) }

fn parse_args() -> Opts {
    let mut a = std::env::args().skip(1);
    let cmd = a.next().unwrap_or_else(|| usage("usage"));
    if !["run", "list", "watch"].contains(&cmd.as_str()) { usage(&format!("unknown command {cmd}")); }
    let mut o = Opts { cmd, cache_dir: PathBuf::from(".c1ft/cache"), no_cache: false, filter: None, every_ms: None, rounds: None, paths: vec![] };
    while let Some(x) = a.next() {
        let mut val = |f: &str| a.next().unwrap_or_else(|| usage(&format!("{f} needs a value")));
        match x.as_str() {
            "--cache-dir" => o.cache_dir = PathBuf::from(val("--cache-dir")),
            "--no-cache" => o.no_cache = true,
            "--filter" => o.filter = Some(val("--filter")),
            "--every-ms" => o.every_ms = Some(val("--every-ms").parse().unwrap_or_else(|_| usage("--every-ms needs a whole number of milliseconds"))),
            "--rounds" => o.rounds = Some(val("--rounds").parse().unwrap_or_else(|_| usage("--rounds needs a whole number"))),
            f if f.starts_with("--") => usage(&format!("unknown option {f}")),
            p => o.paths.push(PathBuf::from(p)),
        }
    }
    if o.cmd == "watch" && o.every_ms.is_none() { usage("watch needs --every-ms <n>: how often the inputs are looked at (c1ft has no default for it)"); }
    if o.paths.is_empty() { o.paths.push(PathBuf::from(".")); }
    o
}

fn find(p: &Path, out: &mut Vec<PathBuf>) {
    if p.is_dir() {
        let mut es: Vec<PathBuf> = std::fs::read_dir(p).map(|r| r.filter_map(|e| e.ok().map(|e| e.path())).collect()).unwrap_or_default();
        es.sort();
        for e in es {
            let n = e.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if e.is_dir() && (n.starts_with('.') || n == "target" || n == "node_modules") { continue; }
            find(&e, out);
        }
    } else if p.extension().map_or(false, |x| x == "c1ft") { out.push(p.to_path_buf()); }
}
fn files(paths: &[PathBuf]) -> Vec<PathBuf> { let mut v = vec![]; for p in paths { find(p, &mut v); } v }

struct Planned { test: format::Test, base: PathBuf }
fn plan(o: &Opts) -> Result<Vec<Planned>, String> {
    let mut v = vec![];
    for f in files(&o.paths) {
        let text = std::fs::read_to_string(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        let base = f.parent().map(Path::to_path_buf).filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| PathBuf::from("."));
        let shown = f.strip_prefix(&o.paths[0]).ok().filter(|p| !p.as_os_str().is_empty()).map(Path::to_path_buf).unwrap_or_else(|| f.file_name().map(PathBuf::from).unwrap_or(f.clone()));
        for t in format::parse(&shown.to_string_lossy(), &text)? { if o.filter.as_ref().map_or(true, |s| t.name.contains(s.as_str())) { v.push(Planned { test: t, base: base.clone() }); } }
    }
    Ok(v)
}

fn run_guarded(t: &format::Test, base: &Path) -> Result<(), String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run::run_test(t, base))) {
        Ok(r) => r,
        Err(p) => Err(format!("{}:{}: the engine panicked: {}", t.file, t.line, p.downcast_ref::<String>().cloned().or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default())),
    }
}

/// One round: every selected test, from the cache or run. Returns (exit code, the inputs and files to watch).
fn round(o: &Opts) -> (i32, Vec<PathBuf>) {
    let planned = match plan(o) { Ok(p) => p, Err(e) => { eprintln!("c1ft: {e}"); return (2, files(&o.paths)); } };
    if planned.is_empty() { eprintln!("c1ft: no test found{}", o.filter.as_ref().map(|f| format!(" whose name holds {f:?}")).unwrap_or_default()); return (2, files(&o.paths)); }
    let _ = std::fs::create_dir_all(&o.cache_dir);
    let (mut passed, mut cached, mut failed) = (0, 0, 0);
    let mut watch = files(&o.paths);
    for p in &planned {
        let t = &p.test;
        if let Ok(ins) = cache::inputs(t, &p.base) { watch.extend(ins); }
        let label = format!("{}: {}", t.file, t.name);
        let key = cache::key(t, &p.base);
        let hit = !o.no_cache && key.as_ref().map_or(false, |k| o.cache_dir.join(k).exists());
        if hit { println!("CACHED {label}"); passed += 1; cached += 1; continue; }
        let t0 = Instant::now();
        match key.and_then(|k| run_guarded(t, &p.base).map(|_| k)) {
            Ok(k) => { let _ = std::fs::write(o.cache_dir.join(k), b"pass\n"); println!("PASS   {label}  ({} ms)", t0.elapsed().as_millis()); passed += 1; }
            Err(e) => { println!("FAIL   {label}\n       {}", e.replace('\n', "\n       ")); failed += 1; }
        }
    }
    println!("c1ft: {} tests: {passed} passed ({cached} cached), {failed} failed", planned.len());
    (if failed > 0 { 1 } else { 0 }, watch)
}

fn snapshot(paths: &[PathBuf]) -> Vec<(PathBuf, Option<SystemTime>, u64)> {
    let mut v: Vec<_> = paths.iter().map(|p| { let m = std::fs::metadata(p).ok(); (p.clone(), m.as_ref().and_then(|m| m.modified().ok()), m.map_or(0, |m| m.len())) }).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v.dedup_by(|a, b| a.0 == b.0);
    v
}

fn main() {
    std::panic::set_hook(Box::new(|_| {}));   // a panic in a test is its failure message (run_guarded), not noise
    let o = parse_args();
    match o.cmd.as_str() {
        "list" => match plan(&o) {
            Ok(p) => { for x in p { println!("{}: {}", x.test.file, x.test.name); } }
            Err(e) => { eprintln!("c1ft: {e}"); std::process::exit(2); }
        },
        "run" => std::process::exit(round(&o).0),
        _ => {
            let every = Duration::from_millis(o.every_ms.unwrap_or(0));
            let mut n = 0u64;
            loop {
                let (code, watch) = round(&o);
                n += 1;
                if o.rounds.map_or(false, |r| n >= r) { std::process::exit(code); }
                let before = snapshot(&watch);
                loop {
                    std::thread::sleep(every);
                    // the inputs of the last round, and any .c1ft file that appeared or went
                    if snapshot(&watch) != before || snapshot(&files(&o.paths)).len() != before.iter().filter(|(p, _, _)| p.extension().map_or(false, |x| x == "c1ft")).count() { break; }
                }
            }
        }
    }
}
