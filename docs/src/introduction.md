# Introduction

Romit is a declarative language for describing state, events, rules, and tests. Programs use the `.rmt` file extension and the `romit` command-line tool.

Romit’s surface syntax is **brace-free**: no `{ }`, `[ ]`, or `( )`. Blocks are shown only by **indentation** (two spaces per level; tabs are rejected with **P001**).

```text
on Run
  rule say_it
    when true
    emit Greet with message "Hello, world!"
```

The compiler in this repository implements **Stage 0–1**: specs, checks, tests, and a small interpreter for beginner examples. UI `app` / `screen` syntax is documented under planned examples.
