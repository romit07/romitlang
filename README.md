# Romit

[![docs](https://img.shields.io/badge/docs-romitlang.org-blue)](https://romitlang.org/)

Romit is a brace-free declarative language (`.rmt` files, `romit` CLI). This repository contains the Stage 0–1 Rust compiler, examples, a marketing home page, browser playground, and [mdBook](https://rust-lang.github.io/mdBook/) documentation.

Current release line: **v0.0.1**

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
romit run examples/beginner/hello.rmt
romit test examples/beginner/hello_world.rmt
```

## Examples

| Directory | Description |
|-----------|-------------|
| `examples/beginner/` | Start here — scripts and specs |
| `examples/advanced/` | Larger specs (brace-free); use `romit check --no-coverage` |
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
