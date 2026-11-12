# Editor support

## VS Code, Cursor, Windsurf, VSCodium

The Romit extension adds syntax highlighting, snippets, a file icon, and Romit's indentation rules (2 spaces, no tabs) for `.rmt` files.

1. Download `romit-<version>.vsix` from the [latest release](https://github.com/romit07/romitlang/releases/latest).
2. Install it:

   ```bash
   code --install-extension romit-0.1.0.vsix
   ```

   In Cursor use `cursor --install-extension …`. You can also use the Extensions view: **⋯** → **Install from VSIX…**.

Type `app` and press Tab to start a new app. Other snippets: `screen`, `state`, `button`, `list`, `foreach`, `if`, `field`, `toggle`, `picker`, `tabs`, `sidebar`, `action`, `fn`, `spec`.

The source is in [`editors/vscode`](https://github.com/romit07/romitlang/tree/main/editors/vscode).

## Other editors

The grammar is a standard TextMate grammar ([`romit.tmLanguage.json`](https://github.com/romit07/romitlang/blob/main/editors/vscode/syntaxes/romit.tmLanguage.json)). Sublime Text, Zed, Nova, and JetBrains IDEs (through the TextMate Bundles plugin) can load it. Set your editor to indent with 2 spaces and never insert tabs.
