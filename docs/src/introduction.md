# Introduction

Romit is a **general-purpose** programming language for **shorter code, where the compiler fills in sensible defaults** — plain statements, indentation only, no braces, brackets, or parentheses. Programs use the `.rmt` file extension and the `romit` command-line tool.

AI-assisted work today means long prompts and big generated codebases. Romit sits in between: a **tiny language** that reads almost like a prompt but executes. Rules, state machines, invariants, and tests are an **optional spec layer** when you want checks — not required for every program.

Romit’s surface syntax is **brace-free**: no `{ }`, `[ ]`, or `( )`. Blocks use **indentation** (two spaces per level; tabs are rejected with **P001**).

The fastest program is a **script**:

```text
say "Hello, world!"
```

```bash
romit run hello.rmt
```

For optional structured outputs and tests, start a file with `spec` — see [Optional spec layer](language/spec-layer.md) and `examples/beginner/hello_world.rmt`.

The compiler in this repository runs scripts (variables, decimals, text interpolation, lists, functions, loops), checks and tests the spec layer, and powers a browser playground. **Romit UI** (`use ui`) builds **reactive SwiftUI-style apps** (state, stacks, controls, navigation, tabs, sheets, saved data) and static pages:

```text
use ui

state
  count is 0

screen Home
  text "Tapped {count} times" bold
  button "Tap me" do count += 1
    style prominent
```

See [Building apps](ui-apps.md).

See [Why Romit](why.md) for motivation and a before/after prompting example.
