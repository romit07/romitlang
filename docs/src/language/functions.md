# Functions and calls

Romit defines functions in **scripts** with a header line and an indented body. The primary form uses a colon after the name:

```romit
sum: a, b
  give a plus b

say sum 3, 4
```

## Definition forms (equivalent)

| Form | Example |
|------|---------|
| Colon (preferred in docs) | `greet: name is "world"` |
| Short alias | `fn greet name is "world"` |
| Longer alias | `function greet name is "world"` |

Parameters are comma-separated; spaces between parameters are also accepted (`fn sum a b`). Defaults use `is`, like fields elsewhere: `fn greet name is "world"`.

No parameters use a bare header line:

```romit
hello:
  give "ready"

fn ping
  give "pong"
```

`define` with `returns` remains the form used inside the optional **spec** layer; script functions use `give` instead of `returns`.

## Calls

Calls never use `:` or `fn` / `function`. Write the name followed by arguments on the same line:

```romit
say sum 3, 4
set x to sum 1, 2
```

Commas separate arguments; spaces alone work too (`sum 3 4`).

Named arguments at the call site use `with`: `greet with name "Ada"`.

## `say` / `print` with several values

When a line uses commas between **separate** printed values, each segment is its own expression:

```romit
say "Total: ", total
say one 1, sum 2, 3
```

In `say one 1, sum 2, 3`, the first part is the call `one 1`, and the second part is `sum 2, 3` (that call takes the rest of the line as its arguments).

Nested calls are only allowed as the **last** argument on a line; otherwise assign to a variable first (see error **P028**).

Functions may appear before or after their use in the same file; the checker resolves every definition.
