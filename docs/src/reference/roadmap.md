# Roadmap

**Goal:** a general-purpose compiled language with **shorter code, where the compiler fills in sensible defaults** — prompt-like readability and one `romit` tool — growing from tiny scripts and optional spec checks toward full applications, native compilation, and a built-in web UI layer.

1. **Stage 1 (current)** — Scripts with `say`, optional spec layer, lexer, parser, checker, tests, mdBook docs, release binaries and install scripts.
2. **Stage 2 (started)** — Script functions (`sum:`, `fn`, `function`), control flow, operator words, comma rules, lists, decimals, and text interpolation; replay tooling and native compilation still planned. See the [Swift feature map](swift-feature-map.md).
3. **Stage 3** — Entities and machines at full fidelity in the interpreter.
4. **Stage 4** — Romit UI: static pages (`page`) and **reactive SwiftUI-style apps shipped** (`app`, `state`, `screen`, layout, controls, navigation, tabs, sheets, saved data). See [Building apps](../ui-apps.md) and [SwiftUI → Romit](swiftui-map.md). Next: split views, charts, drag gestures, and native (iOS/Android) output.

## Distribution (planned)

- **Homebrew tap** — third-party or official `brew install romitlang/tap/romit` once binaries are published; not implemented yet.
- **npm wrapper** — global `npm install -g @romitlang/cli` shim that downloads the matching release; not implemented yet.
