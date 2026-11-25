# The runner

```
c1ft run   [--cache-dir <dir>] [--no-cache] [--filter <text>] [<path>...]
c1ft list  [<path>...]
c1ft watch --every-ms <n> [--rounds <n>] [--cache-dir <dir>] [--no-cache] [--filter <text>] [<path>...]
```

A `<path>` is a `.c1ft` file or a folder searched, in name order, for them (default: the current folder). Folders that start with `.` and the folders `target` and `node_modules` are not searched. Tests run one at a time, in file order.

## Output

One line per test, then a summary:

```
PASS   file.c1ft: name  (12 ms)
CACHED file.c1ft: name
FAIL   file.c1ft: name
       file.c1ft:10: Thing:a.twice reads 4, expected 5
c1ft: 3 tests: 2 passed (1 cached), 1 failed
```

A file is named relative to the first path given. `list` prints `file: name` and runs nothing.

## Exit codes

`0` every test passed (cached or run), `1` a test failed, `2` a mistake: a usage error, a file that does not parse, a path that cannot be read, or a filter or folder that selects no test (that is never a pass).

## Options

- `--cache-dir <dir>`: where passes are kept (default `.c1ft/cache`, under the current folder).
- `--no-cache`: run every test; a pass is still written to the cache.
- `--filter <text>`: only the tests whose name holds the text.
- `--every-ms <n>`: (`watch`) how often the inputs are looked at. **Required**: c1ft has no default for it.
- `--rounds <n>`: (`watch`) stop after `n` runs, with the exit code of the last. Without it `watch` runs until it is stopped.

## Watch

`watch` runs everything once, then looks at the modification time and size of every `.c1ft` file and every file a test loads, every `--every-ms` milliseconds. When any of them changes, or a `.c1ft` file appears or goes, it runs again. The second run takes every test whose inputs did not change from the cache, so only the tests that read the changed file run ([reactive.md](reactive.md)).

## Testing an engine tree

c1ft runs on the cliffvm it was built with, and the cache key hashes the program, so a different engine never shares a result. To test a working tree of cliffvm (its own tests, written as c1ft), build c1ft against it:

```
cargo install --path . --force --config 'patch.crates-io.cliffvm.path="/path/to/cliffvm"'
```

`cargo install --git https://github.com/chedim/c1ft --config ...` does the same from a clone. The tree's version must be inside the range `Cargo.toml` allows (0.4.1 up to, not including, 0.5).
