# Romit

[![docs](https://img.shields.io/badge/docs-romitlang.org-blue)](https://romitlang.org/)

**Write what you mean. Ship it.**

Romit is a general-purpose compiled language (`.rmt` files, `romit` CLI) aimed at **shorter code, where the compiler fills in sensible defaults** — short plain statements, indentation only, no braces, brackets, or parentheses. It reads almost like an AI prompt, but it runs. An optional spec layer adds rules, state machines, invariants, and tests when you want checks.

This repository contains the compiler (implemented in Rust), **Romit UI** for SwiftUI-style reactive apps and static pages (`use ui`, `romit build`), examples, a home page, a browser playground with live app previews, and [mdBook](https://rust-lang.github.io/mdBook/) documentation.

```text
use ui

app Todos
  state
    draft is ""
    todos is list saved

  screen Home "My Todos"
    row
      field "What needs doing?" bind draft
      button "Add" do add draft to todos
        style prominent
        disabled if draft is empty
    list
      for each todo in todos
        text todo
          on delete do remove todo from todos
    caption "{count of todos} to do"
```

`romit build todos.rmt` produces one `index.html` you can open anywhere. It looks native on each device: Apple style on iPhone, iPad and Mac, Material 3 on Android, Windows and Linux (or pick one with `theme` → `look material`). See [Building apps](https://romitlang.org/ui-apps.html) and the [SwiftUI → Romit map](https://romitlang.org/reference/swiftui-map.html).

Current release line: **v0.0.3**

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

Pin version **0.0.3**:

```bash
ROMIT_VERSION=0.0.3 curl -fsSL https://romitlang.org/install.sh | sh
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
romit run examples/beginner/lists.rmt
romit test examples/beginner/hello_world.rmt
romit run examples/apps/todos.rmt        # live preview at http://127.0.0.1:3847
romit build examples/apps/showcase.rmt   # writes examples/apps/dist/index.html
romit build examples/ui/landing.rmt
```

## Examples

| Directory | Description |
|-----------|-------------|
| `examples/beginner/` | Start here — scripts and optional specs |
| `examples/advanced/` | Larger spec programs (brace-free); use `romit check --no-coverage` |
| `examples/apps/` | SwiftUI-style apps: counter, todos, notes, mail, showcase (`romit build` / `romit run`) |
| `examples/ui/` | Static Romit UI pages (`romit build`) |
| `examples/planned/` | Sketches of future syntax |
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
