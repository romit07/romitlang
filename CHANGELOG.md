# Changelog

All notable changes to Romit. Versions follow [semantic versioning](https://semver.org/). While Romit is 0.x, a new middle number means new features.

## v0.1.0: Apps for iPhone and Android

The first release that builds apps. Write a screen the way you'd describe it, and Romit turns it into a reactive app that looks native on every device.

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
```

```bash
romit run todos.rmt      # live preview at http://127.0.0.1:3847
romit build todos.rmt    # one self-contained index.html
```

### Apps (SwiftUI-style)

- `use ui` with `app`, `state`, `computed`, `screen`, `view` and `action` builds reactive apps that redraw when data changes. `saved` keeps data on the device.
- **Layout:** `column`, `row`, `stack`, `grid`, `wrap`, `scroll`, `list`, `section`, `form`, `card`, `box`, `disclosure`, `spacer`, `divider`.
- **Content:** text styles, `label`, `icon` (SF Symbol names), `image`, `video`, shapes, `progress`, `spinner`, `gauge`, `empty`.
- **Controls with `bind`:** `button`, `link`, `toggle`, `field`, `secure`, `editor`, `search`, `slider`, `stepper`, `picker`, `date`, `colorpicker`, `menu`, `share`.
- **About 55 modifiers** (`bold`, `padding`, `color`, `corner`, `shadow`, …), each usable with `if` or `unless`: `strike if todo.done`.
- `if / otherwise` and `for each … where …` inside views.
- **Events:** tap, double tap, long press, hover, submit, change, delete, appear, `every 1 second`.
- **Actions:** change values, edit lists, navigate, `alert`, `ask` (confirm), `toast`, `wait`, `copy`, `share`, `load "url" into x` (JSON).
- **Expressions:** `count of todos where not done`, `sorted by`, `price as money`, `random 1 to 6`.

### Native look on iPhone and Android

- **Apple style** on iPhone, iPad and Mac.
- **Material 3** (Material You) on Android, Windows, Linux and ChromeOS: top app bar, navigation bar, pill buttons, Material switches and fields, bottom sheets, snackbars.
- Pick one with `theme` → `look auto | apple | material`, or add `?look=` to the app's address.
- `style fab` adds a floating action button.

### App structure (from SwiftUI)

| Romit | SwiftUI |
|---|---|
| `sidebar` block | `NavigationSplitView` |
| a URL for every screen; browser back/forward | `NavigationPath`, deep links |
| `show X as half / full / popover / inspector`, `locked` | sheet detents, `.fullScreenCover`, `.popover`, `.inspector`, `interactiveDismissDisabled` |
| `toolbar leading / title / bottom` | toolbar placements |
| `subtitle`, `inline` | `.navigationSubtitle`, inline titles |
| `search … suggest … scopes …` | search suggestions and scopes |
| `on background`, `on foreground`, `on url` | `scenePhase`, `onOpenURL` |
| `screen Prefs settings` | `Settings` scene |
| `open window Screen` | `openWindow` |
| `document a, b`, `open file into x`, `save x to file "…"` | `DocumentGroup`, file importer and exporter |

### Language

- `+` and `plus` now join text. `"a" plus "b"` used to give `0`.
- Decimals work (`1.5` used to become `0`), and division is exact (`7 / 2` is `3.5`).
- `"Hi {name}"` inserts values into text. Braces are allowed only inside text.
- `# comments` and `note` lines work in scripts.
- `x += 1`, `x -= 1`, `increase x by 2`, `decrease x`.
- Lists: `list a, b`, `add x to xs`, `remove x from xs`, `for each x in xs`, `xs contains x`, `item 2 of xs`.
- Built-ins: `count`, `sum`, `average`, `min`, `max`, `first`, `last`, `sort`, `reverse`, `unique`, `join`, `split`, `upper`, `lower`, `trim`, `round`, `floor`, `ceil`, `abs`, `sqrt`, `number`, `text`, `range`, `random`.
- `stop` and `skip` inside loops.
- Fixed: `event … with … stores …` on one line failed to parse.
- Clear errors with "did you mean" hints (codes U101–U118 for apps).

### Tools

- **VS Code extension** with highlighting, snippets and Romit indentation. It works in Cursor, Windsurf and VSCodium; install `romit-0.1.0.vsix` from this release.
- **Playground:** live, clickable previews of `use ui` programs, a Look menu (Apple / Android), and new example apps (Counter, Todos, Notes, Mail, Showcase).
- `romit run app.rmt` re-reads the file on every refresh and shows errors in the page.
- `romit check` understands apps.

### Docs and website

- New guides: [Building apps](https://romitlang.org/ui-apps.html), [SwiftUI → Romit](https://romitlang.org/reference/swiftui-map.html), [Lists, text and numbers](https://romitlang.org/language/lists.html), [Editor support](https://romitlang.org/getting-started/editor.html).
- Fixed the install tabs on romitlang.org, which showed every platform at once.

## v0.0.2

- Script functions (`fn`, `function`, `name:`) with `give`, default parameters, and comma or space arguments.
- Control flow: `if` / `otherwise if` / `otherwise`, `repeat N times`, `for each i in 1 to 5`, `while`.
- Operator words (`plus`, `times`, `is at least`, …) alongside symbols.
- Statement-level `x = 5` assignment, top-level script variables, and `print` as an alias for `say`.
- Romit UI static pages: `use ui` + `page`, built with `romit build`.

## v0.0.1

- First release: brace-free syntax, `say` scripts, the optional spec layer (rules, machines, invariants, tests), `romit run` / `check` / `test`, install scripts, the website, and the WASM playground.
