//! The cache key of a test (docs/reactive.md): a sha-256 over the format version, the engine, the test's own text and, for every file the test loads (a folder: every file under
//! it), the path and the bytes. The same bytes give the same key; a change to any input gives another, so a pass found under a key is a pass for exactly these inputs.
use crate::format::{Load, Test};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn walk(p: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let md = std::fs::metadata(p).map_err(|e| format!("{}: {e}", p.display()))?;
    if md.is_dir() {
        let mut es: Vec<PathBuf> = std::fs::read_dir(p).map_err(|e| format!("{}: {e}", p.display()))?.filter_map(|e| e.ok().map(|e| e.path())).collect();
        es.sort();
        for e in es { walk(&e, out)?; }
    } else { out.push(p.to_path_buf()); }
    Ok(())
}

/// Every file the test reads, in a fixed order: for a `load` the file (or each file of the folder), for a `list` the list file and each pack it names.
pub fn inputs(t: &Test, base: &Path) -> Result<Vec<PathBuf>, String> {
    let mut v = vec![];
    for l in &t.loads {
        match l {
            Load::File(p) => walk(&base.join(p), &mut v).map_err(|e| format!("{}:{}: load {p}: {e}", t.file, t.line))?,
            Load::List { file, dir } => {
                let lp = base.join(file);
                let text = std::fs::read_to_string(&lp).map_err(|e| format!("{}:{}: list {file}: {e}", t.file, t.line))?;
                v.push(lp);
                for n in crate::run::list_names(&text) { walk(&base.join(format!("{dir}/{n}.c1f")), &mut v).map_err(|e| format!("{}:{}: list {file}: {e}", t.file, t.line))?; }
            }
        }
    }
    Ok(v)
}

/// The engine a result was made under: a sha-256 of the running program, which has the engine compiled in. A rebuilt engine (a new release, or the working tree of the engine
/// under test) is another program, so another key. Read once.
pub fn engine_id() -> String {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(|| {
        let bytes = std::env::current_exe().and_then(std::fs::read).unwrap_or_default();
        Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect()
    }).clone()
}

pub fn key(t: &Test, base: &Path) -> Result<String, String> { key_with(t, base, &engine_id()) }

pub fn key_with(t: &Test, base: &Path, engine: &str) -> Result<String, String> {
    let mut h = Sha256::new();
    for part in [crate::FORMAT, engine, &t.text] { h.update((part.len() as u64).to_le_bytes()); h.update(part.as_bytes()); }
    for p in inputs(t, base)? {
        let rel = p.strip_prefix(base).unwrap_or(&p).to_string_lossy().to_string();
        let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        h.update((rel.len() as u64).to_le_bytes()); h.update(rel.as_bytes());
        h.update((bytes.len() as u64).to_le_bytes()); h.update(&bytes);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
