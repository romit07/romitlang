# Romit

[![docs](https://img.shields.io/badge/docs-romitlang.org-blue)](https://romit07.github.io/romitlang/)

Romit is a brace-free declarative language (`.rmt` files, `romit` CLI). This repository contains the Stage 0–1 Rust compiler, examples, and [mdBook](https://rust-lang.github.io/mdBook/) documentation.

## Quick start

```bash
cargo build --release
./target/release/romit test examples/beginner/hello_world.rmt
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
mdbook build docs
```

Published site: [https://romit07.github.io/romitlang/](https://romit07.github.io/romitlang/) (configure GitHub Pages → **GitHub Actions**). Custom domain: `romitlang.org` (`docs/src/CNAME`).

## License

See repository settings; compiler sources are intended for open collaboration on `romitlang`.
