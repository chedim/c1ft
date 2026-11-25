# The cache and the reactive model

## The key

A test's key is a sha-256 over, in order: the format version (`FORMAT`), the engine release (`ENGINE`), the test's own lines, and, for every file the test loads, its path (relative to the test's folder) and its bytes. A `load` of a folder takes every file under it, a `list` takes the list file and every pack it names.

So the same key means the same inputs: the same test text, the same engine, the same packs. A pass found under a key is a pass for exactly those inputs.

- A change to another test of the same file does not change this test's key.
- A change to a file the test does not load does not change it.
- A file that is missing is an error that names it, never a silent key.

## What is cached

Only passes. A failure always runs again. Nothing is remembered about a test that did not pass, so a flaky or newly fixed test is never stuck on an old answer. A cache entry is a file named by the key in the cache folder; deleting the folder is always safe.

## Reactive, as far as it goes

`c1ft watch` is the reactive loop: when an input changes, the tests whose key changed run again and the rest come from the cache. Which tests those are is decided by what each test declares it loads. That is exact for a test that loads what it reads and is **all of them** for a test that loads the whole seat (every pack of the seat's list is an input).

## Not built

- **The scheduler as a cliffvm app.** The runner is a Rust program using the engine as a library. Making each test an entity whose result is a getter, watched by cliffvm's own file watchers, is the design this grew from, and is not written.
- **Tests that start a process.** The test engine has no effects, so a test that starts a seat, talks HTTP or runs a fake CLI cannot be written in c1ft yet. That needs `Command` and `Fetch` in the test engine, a port and readiness rule, and a teardown that cannot leave a process running; each of those has a constant that is the owner's to set.
- **Recording what a test really reads.** Inputs are what a test declares with `load` and `list`. Recording the files a test opens would need operating-system tracing that wants root on macOS, so it is not used.
- **Parallel runs.** Tests run one at a time.
- **A clock, a store, the wasm engine** inside a test.
- **Release.** There is no publish workflow. CI (`.github/workflows/test.yml`) runs `cargo test` on Ubuntu and macOS, on every push and pull request.
