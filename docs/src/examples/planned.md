# Planned UI syntax

The file `examples/planned/landing-dark.rmt` shows a future **app / screen** layer (not parsed by the Stage 1 compiler):

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
