# Planned UI syntax

Most of these sketches are now real: see **[Building apps](../ui-apps.md)** and `examples/apps/`. The rest describe directions that are still open (screen-level `defaults`, per-state button styles).

| File | What it shows |
|------|----------------|
| `counter_ui.rmt` | Shipped; points to `examples/apps/counter.rmt` |
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
