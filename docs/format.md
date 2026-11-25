# The `.c1ft` file

A file holds tests. `test <name>` starts one and a name is unique in its file. Everything before the first `test` must be blank or a `--` comment.

A **directive** is a line that starts with its keyword (`given Thing:a {"n": 2}`). The same line with `-- ` in front (`-- given Thing:a {"n": 2}`) is the same directive: that is the form the help examples use, where a directive has to be a c1f comment, and both forms may share a file. This page writes the bare form.

A line inside a test is a directive when its first word is a keyword (below), and is otherwise part of the test's **pack**: c1f text that is loaded as one pack (`--` comments and all). A pack's own lines start with `PACK`, `IMPORT` or a class name, and its bodies are indented, so none of them starts with a keyword. A `-- ` line whose first word is not a keyword is a comment of the pack.

Each test runs on a fresh engine: no store, no effects, no other test's entities. Paths in a test are relative to the folder of its file.

## Directives

| Directive | Meaning |
|---|---|
| `test <name>` | start a test |
| `load <path>` | load a pack file, or every pack of a folder (an app), before the test's own pack |
| `list <list file> <dir>` | load the packs a list names: one name per line (blank lines and `#` lines skipped), each `<dir>/<name>.c1f`, the first one trusted, as the seat's boot does |
| `given <eid> <json object>` | birth an entity with these properties, then let the engine settle |
| `write <eid>.<prop> = <json>` | write a property, then settle |
| `pump` | settle (the two above already do) |
| `reads <eid>.<prop> = <json>` | the property must read this value |
| `let <name> = <expr>` | a variable for the `eval`s below it ([let](#let)) |
| `type <expr> [with <name>: <Type>, ...] = "<type>"` | the static inference's type of an expression ([engine](#the-engine-itself)) |
| `findings <expr> [with ...] = <n>` | how many problems the static check finds in it |
| `budget <n>` / `budget off` | the evaluation step budget, from here on |
| `clock step <ms>` / `clock off` | a fake clock that advances `ms` at every read, from here on |
| `stats on` / `stats off` | count the engine's evaluations (clears the counters when turned on) |
| `stats <getter\|setter> <Class>.<prop> <field> (=\|>=\|<=) <n>` | a field of a counter row: `calls`, `ns`, `wait_ns` or `steps` |
| `ns <name>` | the engine's own namespace (a page's, say): an id under another namespace is foreign |
| `feed <eid> = {json}` / `feed gone <eid>` / `feed refused <eid> = "<why>"` | what a peer says of a node of its namespace |
| `store` | run the engine on a store file of the test's own |
| `restart` | close the engine and open it again on the same store |
| `replay` | a fresh engine with the same packs replays this one's journal; the steps below run on it ([records](#the-engines-records)) |
| `same <eid>` / `same all` | the entity (every entity) is in the replayed engine what it is in the original |
| `journal only outside` / `journal lacks write <prop>` / `journal has born <eid>` / `journal has write <eid>.<prop>` | a check of the journal, which holds the outside writes only |
| `privileged <pack> = true\|false` | the engine's own privilege flag of a pack |
| `row <eid> has <prop>` / `row <eid> lacks <prop>` | the stored row of an entity |
| `refusals = <n>` / `refusal <n> holds "<text>"` | how many writes the engine refused, and what the n-th said |
| `eval <expr> = <json>` | the expression, evaluated now on the engine, must give this value ([eval](#eval)) |
| `eval <expr> = error: <text>` | the expression must fail, and its reason must hold the text |
| `exists <eid>` / `absent <eid>` | the entity is there / is not |
| `error: <text>` | the test's pack must be refused by the loader, and its message must hold the text |

The last `.` before ` = ` splits an id from its property. The class of an entity is the first word of its id after a leading `<namespace>::` (`membrane::Thing:a` is a `Thing`).

## eval

`eval <expr> = <json>` evaluates a c1f expression as the body of a getter, at that point of the test: after the `given`s and `write`s above it, with the test's pack loaded. The expression is on one line and holds no ` = ` (the first ` = ` ends it). It sees entities by their ids (`Command:x.rc`, `${'Command:x'}['rc']`) and the getters of the test's pack, and nothing else: there are no variables.

`= error: <text>` expects a failure: the loader refuses the expression, or its value is an Error, or a null whose reason says why (`absent`, `pending`). The reason or the printout of the value must hold the text. A value that is no failure fails the step with the value it had.

## let

`let <name> = <expr>` binds a variable that every `eval` below it sees (not the ones above). The expression may use an earlier variable, an entity by its id, or `${...}`. A name is let once per test, as a local is in c1f (the loader refuses a second one); another test has its own variables. A variable that fails makes the evals that use it fail with its reason, and a test can expect that (`eval bad = error: keys`).

```
let xs = [10, 20, 30]
let d = {'a': 1, 'b': ['x']}
eval xs[-1] = 30
eval len(xs) + d['a'] = 4
```

## The engine itself

These say something about the engine, not about a graph.

- **`type`** and **`findings`** ask the static inference (the engine's type checker, the source of `types.json`): `type Int(x) with x: Text = "Int|Error"` says that `Int(x)` has that type when `x` is a `Text`; `findings <expr> = 1` says the checker reports one problem. A type with a comma inside parentheses is one type; `with` separates the variables by a comma outside them.
- **`budget`** limits the steps of every evaluation from that line on. An evaluation the budget stops keeps a null and says why in the entity's `failed`; an `eval ... = error: <text>` matches that too. `budget off` lifts it.
- **`clock step <ms>`** replaces the engine's clock with one that advances by `ms` at every read, so what is measured in time (the slow threshold, the `ns` of a counter row) is what the test says. `clock off` puts the real one back.
- **`stats on`** clears and turns on the engine's evaluation counters; `stats getter Box.fine calls = 1` reads a row. A row is found by its kind (`getter` or `setter`), class and property; a missing row (the stats were off, or nothing ran) is a failure that says so. The counters are one table per process: c1ft runs tests one at a time, and a test that turned them on turns them off when it ends.
- **`ns`** sets the engine's own namespace (the seat's is `membrane`; a page's is something else). An id under another namespace is foreign: a read of it is pending until a peer says something.
- **`feed`** plays that in: `feed peer::Thing:a = {"n": 1}` gives the node those properties, `feed gone peer::Thing:a` says it is not there, `feed refused peer::Thing:a = "too large"` says the peer refused it.
- **`store`** and **`restart`**: with `store` the engine runs on a store file in a folder of the test's own (removed when the test ends). `restart` closes the engine and opens it again on that file: the packs are loaded again, the stored entities are loaded, and the graph settles. Without `store` a restart opens an empty engine.

## The engine's records

These read what the engine keeps about itself.

- **`replay`** makes a fresh engine (the same packs, rebuilt) that replays the journal of the current one, settles, and becomes the engine the steps below run on; the original is kept. **`same <eid>`** says the entity is identical in the two, **`same all`** says every entity is (an entity a test birthed after the replay is not in the original, so it differs). A `same` with no `replay` before it is a failure that says so. A replay is always in memory, whatever `store` says.
- **`journal`** checks the journal: `only outside` says every line comes from an outside writer (a person, the door), as the journal holds nothing a setter or getter derived; `lacks write <prop>` says no line writes the property; `has born <eid>` and `has write <eid>.<prop>` say a line does.
- **`privileged <pack> = true|false`** reads the engine's own flag (what a rule B check consults), which a grant by `write Pack:<name>.privileged = true` sets and a revoke takes back.
- **`row <eid> has|lacks <prop>`** reads the stored row of the entity: what is stored, never a getter's value.
- **`refusals = <n>`** counts the writes the engine refused and **`refusal <n> holds "<text>"`** reads what the n-th said (the receipt names the setter and the pack).

Budget, clock and stats are put back when the test ends, so they do not reach the next one.

## What a read compares

`reads` takes the value the engine holds for the property (a getter's value included) and compares it with the JSON. Numbers compare by value (`4` equals `4.0`). A property the entity does not have reads `null`. The reason a null carries (`absent`, `pending`) is not visible here: to test one, write a getter that asks (`why(...) is Absent`) and read the getter.

## Failing

A test fails with one message that names the file and line: `f.c1ft:7: Thing:a.twice reads 4, expected 5`. It also fails when:

- a loaded pack, or the test's own pack, is refused by the loader (the loader's message is the failure), unless `error:` expects it;
- a pack file is missing;
- the engine panics.

A line that is not understood is a parse error that names the file and line, and the run stops with exit code 2 before any test runs.

## Loading rules that come from the engine

- A pack file's name is its `PACK` name: `bt.c1f` holds `PACK bt 1`.
- A pack that adds getters to a class another pack declares needs `IMPORT` of that pack; the loader says so and the test fails with that message.
- A pack that is already loaded under that name is skipped.

## What a test cannot do

A test runs without effects, so `Command`, `Fetch`, `File` and cron do not run. There is no second process and no wasm engine. Tests that need those stay in the host language ([migrating.md](migrating.md)).
