# Declarative layer

Stage 1 specs use the same brace-free style:

| Keyword | Role |
|---------|------|
| `spec` | Name and version |
| `state` / `config` | Fields with `name type is default` |
| `entity` | `entity Name keyed by id` and indented fields |
| `event` | `event E with arg type` |
| `output` | Declared outputs with indented parameters |
| `on` | Handlers: `on E with args if condition` |
| `rule` | `when` / `otherwise` and `set` / `emit` / `reject` |
| `machine` | `from to to` transitions |
| `invariant` / `limit` / `test` | Properties and tests |

Tests interleave `send` and `expect` lines in order:

```text
test adds_one
  send Tick
  expect count is 1
  send Tick
  expect count is 2
```
