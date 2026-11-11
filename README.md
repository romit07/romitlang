# Romit

[![docs](https://img.shields.io/badge/docs-romitlang.org-blue)](https://romitlang.org/)

Romit is a brace-free declarative language (`.rmt` files, `romit` CLI). This repository contains the Stage 0–1 Rust compiler, examples, and [mdBook](https://rust-lang.github.io/mdBook/) documentation.

## Install

**macOS / Linux**

```bash
curl -fsSL https://romitlang.org/install.sh | sh
```

**Windows**

```powershell
irm https://romitlang.org/install.ps1 | iex
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
romit test examples/beginner/hello_world.rmt
```

## Examples

| Directory | Description |
|-----------|-------------|
| `examples/beginner/` | Start here — all pass `romit test` |
| `examples/advanced/` | Larger specs (brace-free); use `romit check --no-coverage` |
| `examples/planned/` | Future UI syntax (`landing-dark.rmt`) |
| `examples/negative/` | Parser diagnostics (**P013**, **P001**, …) |

## Documentation

Build locally:

```bash
cargo install mdbook
cp install.sh install.ps1 docs/src/
mdbook build docs
```

Published site: [https://romitlang.org/](https://romitlang.org/) via GitHub Pages (`docs/src/CNAME`).

## License

MIT — see [LICENSE](LICENSE).
