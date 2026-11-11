# Planned UI syntax

See the full design draft: **[UI framework](../ui-framework.md)** (planned, not implemented).

| File | What it shows |
|------|----------------|
| `counter_ui.rmt` | Minimal counter; plain `"+"` button uses **app theme** |
| `theme_cascade.rmt` | **App** `theme`, **defaults**, two screens (one overrides **screen defaults**) |
| `landing-dark.rmt` | Marketing layout and navigation phrases |

`examples/planned/landing-dark.rmt` (excerpt— not parsed by the Stage 1 compiler):

```text
app Landing
  theme dark
    background  #0a0a0a
    text        #f5f5f5

screen Home at /
  hero
    text "Build faster with Acme" size 48 bold
    row
      button "Get started"
        on click go to /signup
```

Navigation uses phrases such as `go to /signup` and `on click go to …` instead of arrows or braces.
