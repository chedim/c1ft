# Moving Rust tests over

A Rust engine test that has this shape moves directly:

```rust
let h = Host::open(opts).unwrap();
h.load_pack("bt", PACK).unwrap();  h.rebuild();
h.born("Reader:r1", "Reader", props(json!({"target": "Thing:t1"})), "person");
assert_eq!(h.get("Reader:r1", "one"), json!(4));
```

```
-- test ...
-- load bt.c1f
-- given Reader:r1 {"target": "Thing:t1"}
-- reads Reader:r1.one = 4
```

| Rust | c1ft |
|---|---|
| `h.load_pack(name, text)` of the test's own pack | the pack text as lines of the test |
| `h.load_app(path, trusted)` | `-- load <path>` (never trusted) or `-- list` (the first pack trusted) |
| `h.born(eid, cls, props, "person")` | `-- given <eid> <props>` |
| `h.write(eid, prop, v, "person")` | `-- write <eid>.<prop> = <v>` |
| `h.pump()` | `-- pump` (given and write pump already) |
| `assert_eq!(h.get(eid, prop), v)` | `-- reads <eid>.<prop> = <v>` |
| `h.exists(eid)` | `-- exists <eid>` / `-- absent <eid>` |
| a loader `Err` the test expects | `-- error: <text>` |

`examples/bare_twin/` is cliffvm's `tests/bare_twin_1143.rs` converted: seven tests, one pack file, one `.c1ft` file. It fails the same four tests as the Rust version on the engine that lacks the feature.

## What stays Rust, or waits

- Tests that start `cliffvm serve` or any process, talk over a socket, or use a fake CLI (about half of babyseat's seat tests): [reactive.md](reactive.md#not-built).
- Tests that need the store, a clock, effects, or the wasm engine.
- Tests of the host's own Rust code (`cliffvm`'s drivers, loaders and CLI).
- A few Rust tests of the engine itself should stay as a check that does not depend on the engine being right: a test written in c1f runs on the engine it tests.

## How to convert

One test at a time: write the c1ft test, see it fail where the old behaviour is wrong (run it against an engine without the change, as `examples/` does), see it pass, then delete the Rust test in the same change.
