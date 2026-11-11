# Introduction

Romit is a declarative language for describing state, events, rules, and tests. Programs use the `.rmt` file extension and the `romit` command-line tool.

Romit’s surface syntax is **brace-free**: no `{ }`, `[ ]`, or `( )`. Blocks are shown only by **indentation** (two spaces per level; tabs are rejected with **P001**).

The fastest hello world is a **script** — no `spec`, `on`, or `rule`:

```text
say "Hello, world!"
```

```bash
romit run hello.rmt
```

For full specs with outputs and tests, use the declarative form:

```text
on Run
  rule say_it
    when true
    emit Greet with message "Hello, world!"
```

The compiler in this repository implements **Stage 0–1**: specs, checks, tests, scripts with `say`, a browser playground, and a small interpreter for beginner examples. UI `app` / `screen` syntax is documented under planned examples.

See [Why Romit](why.md) for motivation and an honest comparison with Python, Rust, and Go.
