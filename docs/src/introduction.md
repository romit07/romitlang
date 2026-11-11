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

The compiler in this repository implements **Stage 0–1**: scripts with `say`, a checker and tests for the spec layer, a browser playground, and a small interpreter. **Romit UI** (`use ui`, `romit build`) ships an MVP for static pages; richer `app` / `screen` syntax remains under `examples/planned/`.

See [Why Romit](why.md) for motivation and a before/after prompting example.
