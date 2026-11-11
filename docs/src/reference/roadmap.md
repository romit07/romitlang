# Roadmap

**Goal:** a general-purpose compiled language with **shorter code, where the compiler fills in sensible defaults** — prompt-like readability and one `romit` tool — growing from tiny scripts and optional spec checks toward full applications, native compilation, and a built-in web UI layer.

1. **Stage 1 (current)** — Scripts with `say`, optional spec layer, lexer, parser, checker, tests, mdBook docs, release binaries and install scripts.
2. **Stage 2** — Functions, richer expressions, replay tooling.
3. **Stage 3** — Entities and machines at full fidelity in the interpreter.
4. **Stage 4** — Romit UI: **MVP shipped** (`use ui`, `romit build`, static HTML/CSS); reactive apps and `app` / `screen` syntax still planned ([UI framework](../ui-framework.md), `examples/planned/`).

## Distribution (planned)

- **Homebrew tap** — third-party or official `brew install romitlang/tap/romit` once binaries are published; not implemented yet.
- **npm wrapper** — global `npm install -g @romitlang/cli` shim that downloads the matching release; not implemented yet.
