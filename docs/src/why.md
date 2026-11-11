# Why Romit

Romit is a **general-purpose compiled language** whose goal is **shorter code, where the compiler fills in sensible defaults** — short plain statements, indentation only, no braces, brackets, or parentheses, so you state intent instead of boilerplate. It is meant to read almost like an AI prompt, but it runs.

## The problem with prompting today

Building software with AI today still means **long prompts** and **large piles of generated code** with little structure:

- The prompt is long, vague, and different every time.
- The model rewrites whole files.
- Fixing one thing quietly breaks another.
- Nobody can **see or review** what the program is supposed to do in one short place.

You babysit generated output instead of owning a tiny source file.

## What Romit changes

Romit is a **tiny readable language between a prompt and a program**. You — or an AI — write very little: often one `say` line, or a short script. Humans and models can read the whole thing in a glance.

When you **want** extra safety, you can add an optional **spec layer**: state, events, rules, state machines, invariants, and tests — checked before run. That layer is there when you need it; it is not what Romit is “about” day to day.

**Romit UI** (`use ui`, `romit build`) ships an MVP for static pages today; reactive apps and APIs remain **planned** — see [UI framework](ui-framework.md), `examples/planned/`, and the [roadmap](reference/roadmap.md).

## Before and after

**A long English prompt** might say: Write a small console program for me. When I run it, it should print exactly the line Hello from Romit and nothing else. Do not add a main function, build files, or dependencies I did not ask for. Keep it minimal.

**A few lines of Romit** — runs today with `romit run`:

```text
say "Hello from Romit"
```

This file is `examples/beginner/write_what_you_mean.rmt`.

### Optional checks

When you want tests and structured behavior, add a **spec** — still short:

```text
spec HelloWorld version 1
about "Say hello — optional spec with a test."

output Greet
  message text

event Run

on Run
  rule say_it
    when true
    emit Greet with message "Hello from Romit"

test says_hello
  send Run
  expect emitted Greet with message "Hello from Romit"
```

See `examples/beginner/hello_world.rmt` and run `romit test` on that file. Do not mix `say` scripts and `spec` in one file. UI pages use `use ui` in separate files.

## Reads like a prompt

Romit uses **brace-free, bracket-free, parenthesis-free** syntax with indentation and plain words — `say`, `set answer to`, and when you use the spec layer, `when`, `rule`, `emit Greet with message`. Defaults carry the boilerplate so the source stays short.

## AI-friendly workflow

An assistant edits **very little source**; you run `romit run` or `romit test`. You are not maintaining huge generated trees that drift from intent.

## Memory and apps

The target memory model is **value semantics with automatic reference counting** — **planned**. **Romit UI** (separate from the core language) builds static pages with `romit build` today; reactive screens and backend APIs remain **planned**.

## One tool

The `romit` binary **run**s scripts, **check**s and **test**s specs when you use that layer, **build**s UI pages, and previews them locally. One compiler, one workflow.

## Capability snapshot (honest)

| Area | Today | Planned |
|------|-------|---------|
| Short scripts with `say` | Yes | Richer expressions and functions |
| Brace-free surface | Yes | Same |
| Optional spec layer: rules, machines, invariants, tests | Stage 0–1 | Broader runtime coverage |
| Catch contradictory specs early | Where implemented in checker | Stricter coverage |
| Interpreter | Yes | AOT compiler and native binaries |
| Static web UI (`romit build`) | MVP | Reactive UI and APIs |
| Single `romit` tool | Yes | Same |

**Today:** Romit is strongest for **minimal scripts** and **small spec programs** with tests, plus **static UI pages**. Larger apps, native compilation, and reactive UI are **planned**.
