# Romit for Visual Studio Code

Syntax highlighting, snippets, and editor settings for [Romit](https://romitlang.org/), the language where you write apps the way you write prompts.

```text
use ui

app Counter
  state
    count is 0

  screen Home
    text "Tapped {count} times" bold
    button "Tap me" do count += 1
      style prominent
```

## Features

- **Highlighting** for `.rmt` files: keywords, UI elements, modifiers, actions, `"text with {holes}"`, numbers, colors, and `#` / `note` comments. Brackets outside text are marked as errors, as they are in Romit.
- **Romit-friendly settings**: 2-space indentation and spaces only (tabs are a compile error in Romit), comment toggling with ⌘/ / Ctrl+/, auto-closing quotes, and indentation-based folding.
- **Snippets**: `app`, `screen`, `screenwith`, `state`, `computed`, `button`, `buttonon`, `list`, `foreach`, `if`, `field`, `toggle`, `picker`, `tabs`, `sidebar`, `action`, `fn`, `spec`.
- A file icon for `.rmt` files.

It also works in Cursor, Windsurf, VSCodium, and other VS Code-based editors.

## Install

From a release: download `romit-<version>.vsix` from [GitHub Releases](https://github.com/romit07/romitlang/releases), then:

```bash
code --install-extension romit-0.1.0.vsix
```

Or in VS Code: **Extensions** → **⋯** → **Install from VSIX…**.

To build it yourself:

```bash
cd editors/vscode
npm install
npm test          # tokenizes every example with VS Code's grammar engine
npm run package   # writes romit-<version>.vsix
```

## Develop

`syntaxes/romit.tmLanguage.json` is generated. Element and modifier names come from the compiler's vocabulary (`crates/romit_ui/src/app/vocab.rs`), so after adding an element or modifier to the language, run:

```bash
npm run build-grammar && npm test
```

## Links

- [Language guide](https://romitlang.org/)
- [Building apps](https://romitlang.org/ui-apps.html)
- [Playground](https://romitlang.org/playground/)
