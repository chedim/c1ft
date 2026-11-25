# c1ft

Tests written in c1f, run on the [cliffvm](https://github.com/chedim/cliffvm) engine.

A test file is plain text: a pack (the c1f under test), some entities, writes and the values they must read. `c1ft` loads each test into a fresh engine, runs the steps, and keeps a pass under a key made of everything the test reads, so a change reruns only the tests it touches.

```
-- test a write changes what a getter reads
PACK p 1
Thing.twice getter: this.n * 2
-- given Thing:a {"n": 2}
-- reads Thing:a.twice = 4
-- write Thing:a.n = 5
-- reads Thing:a.twice = 10
```

```
$ c1ft run
PASS   doubles.c1ft: a write changes what a getter reads  (4 ms)
c1ft: 1 tests: 1 passed (0 cached), 0 failed
$ c1ft run
CACHED doubles.c1ft: a write changes what a getter reads
c1ft: 1 tests: 1 passed (1 cached), 0 failed
```

## Use

```
cargo build --release          # the program is target/release/c1ft
c1ft run [path...]             # every .c1ft file under the paths (default: here)
c1ft list [path...]            # the tests, nothing run
c1ft watch --every-ms 500      # run, then run again whenever an input changes
```

`examples/bare_twin/` is cliffvm's `tests/bare_twin_1143.rs` (7 tests) written as c1ft: it passes on cliffvm 0.4.1 and fails the same four tests on 0.4.0, which does not have the feature.

## Docs

[docs/README.md](docs/README.md) is the index: [the file format](docs/format.md), [the runner](docs/runner.md), [the cache and the reactive model](docs/reactive.md) (what is built and what is not), and [moving Rust tests over](docs/migrating.md).

## Status

0.1.0, not published. The engine is cliffvm 0.4 (any release from 0.4.1, or a working tree patched in: [docs/runner.md](docs/runner.md#testing-an-engine-tree)). `cargo test` runs the suite, and CI runs it on Ubuntu and macOS.
