# Planned UI syntax

See the full design draft: **[UI framework](../ui-framework.md)** (planned, not implemented).

The file `examples/planned/counter_ui.rmt` is the smallest interactive demo—state, a `text` bound to `count`, and a `"+"` button. `examples/planned/landing-dark.rmt` shows a richer **app / screen** layout (not parsed by the Stage 1 compiler):

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
