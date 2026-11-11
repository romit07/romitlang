# Romit

[![docs](https://img.shields.io/badge/docs-romitlang.org-blue)](https://romitlang.org/)

**Write what you mean. Ship it.**

Romit is a general-purpose compiled language (`.rmt` files, `romit` CLI) aimed at the **shortest possible code** — short plain statements, indentation only, no braces, brackets, or parentheses. It reads almost like an AI prompt, but it runs. An optional spec layer adds rules, state machines, invariants, and tests when you want checks.

This repository contains the Stage 0–1 compiler (implemented in Rust), examples, a home page, browser playground, and [mdBook](https://rust-lang.github.io/mdBook/) documentation.

Current release line: **v0.0.1**

See [Why Romit](https://romitlang.org/why.html) for motivation and a before/after example.

## Install

**macOS / Linux**

```bash
curl -fsSL https://romitlang.org/install.sh | sh
```

**Windows**

```powershell
irm https://romitlang.org/install.ps1 | iex
```

Pin version **0.0.1**:

```bash
ROMIT_VERSION=0.0.1 curl -fsSL https://romitlang.org/install.sh | sh
```

Binaries are built for Linux `x86_64` and `aarch64` GNU, macOS Intel and Apple Silicon, and Windows `x86_64`. See [Install](https://romitlang.org/install.html) on the docs site for manual downloads, checksum verification, PATH setup, updates, and uninstall.

**From source**

```bash
git clone https://github.com/romit07/romitlang.git
cd romitlang
cargo build --release
./target/release/romit --version
```

## Quick start

```bash
romit run examples/beginner/write_what_you_mean.rmt
romit test examples/beginner/hello_world.rmt
```

## Examples

| Directory | Description |
|-----------|-------------|
| `examples/beginner/` | Start here — scripts and optional specs |
| `examples/advanced/` | Larger spec programs (brace-free); use `romit check --no-coverage` |
| `examples/planned/` | Future UI syntax (`landing-dark.rmt`) |
| `examples/negative/` | Parser diagnostics (**P013**, **P001**, …) |

## Documentation

Build the full site (docs + home + playground WASM):

```bash
./scripts/build_site.sh
```

Docs only:

```bash
cargo install mdbook
cp install.sh install.ps1 docs/src/
mdbook build docs
```

Published site: [https://romitlang.org/](https://romitlang.org/) via GitHub Pages (`docs/src/CNAME`).

## License

MIT — see [LICENSE](LICENSE).
