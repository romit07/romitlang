# Why Romit

Romit is a **general-purpose compiled language** that Romit Kumar is building to eventually replace Python, Rust, and Go for everyday work. It did not start as a language exercise — it started from a **rules-driven trading bot** where fixing one rule often broke another. Rules, stops, and state lived as scattered imperative code, so there was no way to see whole behavior or prove a change was safe.

## The specification is the program

**Core idea:** what you write is the specification *and* the program. Rules, state machines, **invariants** (always-true checks), and **tests** are first-class constructs, validated before anything runs. Contradictory or unsafe programs are rejected with clear errors instead of failing in production.

## Readable surface

Romit uses **brace-free, bracket-free, parenthesis-free** syntax with indentation and plain words (`when`, `rule`, `set answer to`, `emit Greet with message`). The goal is source that non-programmers and AI assistants can read and edit.

## AI-friendly workflow

An assistant can edit the **readable source**; the **compiler checks** it. You are not maintaining fragile generated code that drifts from intent.

## Memory and apps (targets)

The target memory model is **value semantics with automatic reference counting**. A **built-in app framework** for web UI and backend APIs (SwiftUI/React style) is **planned** — see [UI framework](ui-framework.md) and `examples/planned/`.

## One tool

The `romit` binary **check**s, **test**s, and **run**s (scripts). One compiler, one workflow.

## Comparison (honest)

| | Romit today | Python | Rust | Go |
|---|-------------|--------|------|-----|
| Rules, invariants, tests in the language | Yes (Stage 0–1) | No (libraries) | No | No |
| Reject contradictory specs at compile time | Yes, where implemented | Rare | Partial (types) | Partial |
| Readable, brace-free surface | Yes | Yes | No | No |
| Mature package ecosystem | **Planned / early** | Excellent | Excellent | Excellent |
| Production backends at scale | **Not yet** | Yes | Yes | Yes |
| Native code / AOT | **Planned** (interpreter today) | Via extensions | Yes | Yes |
| Built-in web UI framework | **Planned** | Many frameworks | Many crates | Many frameworks |

**Today:** Romit is strong for **spec-like programs** with tests and a small interpreter. It is **not** a drop-in replacement for Python/Rust/Go yet.

## Scripts vs specs

For the smallest programs, use a **script** — one line is enough:

```text
say "Hello, world!"
```

Full **spec** programs keep `on`, `rule`, `emit`, and `test` for advanced behavior. Do not mix styles in one file.
