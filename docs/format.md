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

A test runs without effects, so `Command`, `Fetch`, `File` and cron do not run. There is no clock to advance, no store, no second process, and no wasm engine. Tests that need those stay in the host language ([migrating.md](migrating.md)).
