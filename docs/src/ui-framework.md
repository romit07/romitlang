# UI framework

> **Planned — not implemented yet.**  
> Stage 1 of the `romit` compiler does **not** parse or compile UI specs. This page is a **design draft** only. Treat `examples/planned/*.rmt` as syntax sketches for a future Stage 4 toolchain.

## What we are aiming for

Romit’s UI layer is a **declarative, component-based** framework in the spirit of **SwiftUI** and **React**:

- You describe **what** the screen should look like and **what** happens on events.
- You do not manually wire DOM nodes or imperatively repaint the page.
- **Reactive `state`** drives the UI: when state changes, the visible screen updates automatically (same mental model as `@State` in SwiftUI or `useState` in React).

The compiler pipeline we plan (later) is:

1. Read the UI section of a `.rmt` file (views, layout, modifiers, events).
2. Emit **static HTML** for structure and accessibility.
3. Attach a small **reactive JavaScript** runtime that re-renders only what changed when Romit state updates.

None of that compilation exists in this repository today.

## Core ideas

### `view` components and parameters

A **view** is a named piece of UI. Views can take **parameters** (like SwiftUI’s struct properties or React props) using the same brace-free phrase style as the rest of Romit:

```text
view Badge label text small bool is false
  text label
    size 14
    muted small
```

Parameters are declared on the header line; the indented block is the view body. Composition works by nesting views inside layout containers.

### Reactive `state`

Screen-local or app-wide **state** is declared next to the screen (or in a dedicated `state` block). When a rule or handler runs `set count to count plus 1`, the UI layer should **re-bind** the `text` that reads `count`—no manual “refresh” call.

```text
screen Counter at /counter
  state
    count int is 0
```

This parallels:

| Framework | Reactive state |
|-----------|----------------|
| SwiftUI | `@State`, `@Observable` |
| React | `useState`, `useReducer` |
| Romit (planned) | `state` fields + automatic view updates |

### Layout containers

Nested layout is indentation-only, same as the declarative spec layer:

| Container | Role |
|-----------|------|
| `column` | Vertical stack (like `VStack` / a flex column) |
| `row` | Horizontal stack (like `HStack` / a flex row) |
| `list` | Scrollable list of items |

```text
column
  text "Title" bold
  row
    button "Cancel" outline
    button "OK"
```

### Elements

Basic building blocks:

| Element | Purpose |
|---------|---------|
| `text` | Labels and copy; can show a string or a state field |
| `button` | Tap target with a label |
| `input` | Text field bound to state (planned) |

Strings use `"..."`. State references use plain names: `text count` shows the current value of `count`.

### Modifiers

Modifiers are **trailing words** on a line (SwiftUI-style), not chained punctuation:

```text
text "Hello" size 16 bold
text count color accent padding 12
```

Common modifiers in the draft: `padding`, `color`, `size`, `bold`, `muted`, `outline`, `gap` on containers.

You do **not** have to set every visual property by hand. Anything you leave out is filled in from the **theme** and **defaults** cascade (below). A plain `button "Save"` with no modifiers should still look like a proper button.

### Events

User gestures use phrase events—no parentheses:

```text
button "+"
  on click increment

on click increment
  set count to count plus 1
```

Navigation stays phrase-based too: `on click go to /settings` (see `examples/planned/landing-dark.rmt`).

## App `theme` and built-in defaults

Romit UI is designed so **beginners can skip styling** until they care about branding.

An **`app`** block names the program’s UI shell:

- **`home`** — which `screen` is the entry route when the app loads
- **`theme`** — app-wide visual tokens and control styles
- **`defaults`** — default **screen** settings applied to every screen unless a screen overrides them

The **`theme`** holds colours, type, spacing, corner radius, and interactive **states** for controls (for example `button hover`, `button pressed`, `button focus`, `button disabled`). If you do not specify something on a `text`, `button`, or container, the runtime looks it up here.

**Built-in defaults** sit underneath the theme: readable contrast, minimum tap targets, and sensible fonts if the app does not define a `theme` at all.

### Cascade (who wins?)

When the compiler resolves how something should look, it walks from **most specific** to **least**:

1. **Element override** — modifiers on one line, e.g. `text "Hi" color red`
2. **Screen `defaults`** — the `defaults` block inside that `screen`
3. **App `defaults`** — the `defaults` block directly under `app` (shared by all screens)
4. **App `theme`** — tokens and per-control theme sections (`button`, `input`, …)
5. **Built-in defaults** — what Romit ships when nothing else applies

Think of it like CSS inheritance with a design system: local tweak beats page layout beats design tokens beats the browser baseline.

```text
app CounterDemo
  home Counter
  theme default
    accent #7c3aed
    button
      background accent
      text white
  defaults
    padding 20
    background #f4f4f5
```

A `button "+"` with **no** colour or padding lines still gets the themed button background and label colour.

### Screen `defaults`

Each **`screen`** can rely on app-level `defaults`, or override them with its own `defaults` block. Planned screen-level keys include:

| Key | Purpose |
|-----|---------|
| `background` | Page background (often a theme token name like `background`) |
| `padding` | Inner padding around the main content |
| `safe` | Respect safe areas (`top`, `bottom`, or `top bottom`) |
| `text style` | Default typography for body copy |
| `title style` | Style for the screen title |
| `scroll` | Scrolling behaviour (`vertical`, `horizontal`, `none`) |
| `align` | Default alignment for the main column |
| `layout` | Responsive hint (`stack`, `split`, `fluid` — draft) |
| `empty` | Message when there is no content |
| `loading` | Message while data is loading |
| `error` | Message when something failed |

Only the keys you list need to appear; everything else falls through the cascade.

```text
screen About at /about
  defaults
    padding 32
    background white
  column
    text "About"
```

Here `padding` and `background` override the app `defaults`; button colours still come from `theme`.

### Planned example: theme + two screens

See [`examples/planned/theme_cascade.rmt`](https://github.com/romit07/romitlang/blob/main/examples/planned/theme_cascade.rmt):

- **`Counter`** — `button "+"` has **no** styling modifiers; it uses **`theme`** accent colours and interaction states.
- **`About`** — overrides **`defaults`** (`padding`, `background`) only; buttons would still use the app theme.

## Example: counter (Romit draft)

Full planned example: [`examples/planned/counter_ui.rmt`](https://github.com/romit07/romitlang/blob/main/examples/planned/counter_ui.rmt).

```text
note PLANNED — UI counter demo. Not parsed by Stage 1 romit.

app CounterDemo
  home Counter
  theme default
    accent #2563eb
    button
      background accent
      text white
  defaults
    padding 24

  screen Counter at /
    state
      count int is 0
    column
      gap 16
      text count
        size 48
      button "+"
        on click increment
    on click increment
      set count to count plus 1
```

- **`state`**: `count` starts at `0`.
- **`text count`**: displays the live value (you may still use `size` on the label; the button does not need colours).
- **`button "+"`**: no `background` or `color` lines—the **`theme`** supplies them; `on click increment` raises the event.
- **Handler**: one line updates state; the UI runtime (future) redraws the text.

## Same counter in SwiftUI and React

These snippets are the **same app** in today’s frameworks—useful when you learn Romit’s planned syntax.

### SwiftUI

```swift
import SwiftUI

struct CounterView: View {
    @State private var count = 0

    var body: some View {
        VStack(spacing: 16) {
            Text("\(count)")
                .font(.largeTitle)
                .bold()
            Button("+") {
                count += 1
            }
        }
        .padding(24)
    }
}
```

SwiftUI keeps `count` in `@State` and rebuilds `body` when it changes. Romit’s draft uses an explicit `state` block and rules instead of a closure on the button.

### React

```jsx
import { useState } from "react";

export function Counter() {
  const [count, setCount] = useState(0);

  return (
    <div style={{ padding: 24, textAlign: "center" }}>
      <p style={{ fontSize: 48, fontWeight: "bold" }}>{count}</p>
      <button onClick={() => setCount(count + 1)}>+</button>
    </div>
  );
}
```

React stores `count` in component state and re-renders on `setCount`. Romit’s planned model moves the increment into a declarative `on click` handler that `set`s spec state, closer to the **event / rule** style you already use in beginner `.rmt` files.

## How compilation might work (future)

This is **not built**; it explains the target so tooling can align later.

```mermaid
flowchart LR
  subgraph source [Romit source]
    UI[app / screen / view]
    Rules[on click / state rules]
  end
  subgraph out [Output]
    HTML[HTML skeleton]
    JS[Reactive JS bundle]
  end
  UI --> HTML
  Rules --> JS
  state[(state)] --> JS
  JS --> HTML
```

- **HTML**: semantic tags from `column`, `row`, `text`, `button`, `input`; modifiers become classes or inline data attributes.
- **JS**: a tiny runtime subscribes to state keys (`count`, etc.) and updates text nodes and attributes when handlers fire.
- **Shared rules**: the same `romit test` / interpreter ideas from Stage 1 can eventually drive both server logic and UI events.

## Where to go next

- Theme cascade + two screens: `examples/planned/theme_cascade.rmt`
- Landing page draft: `examples/planned/landing-dark.rmt`
- Shorter planned overview: [Planned UI syntax](examples/planned.md)
- Roadmap: [Roadmap](reference/roadmap.md) (Stage 4)

When the UI compiler lands, this page will move out of “planned” and gain a tutorial that runs `romit build` (or similar) on `counter_ui.rmt`. Until then, **`romit check` will not accept UI-only files** as valid Stage 1 programs.
