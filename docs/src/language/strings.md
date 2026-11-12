# Strings and interpolation

Put a value inside text with `{…}`, the way you would write it in a prompt:

```text
name = "Ada"
say "Hello, {name}! You have {count of tasks} tasks."
```

Anything that works as an expression works inside the braces: `"{price * 2}"`, `"{upper name}"`. Write `{{` or `}}` for a literal brace. Braces are only allowed **inside text**. Code structure stays brace-free (**P013**).

You can also join text with `+` or `plus`:

```text
say "Hello, " + name
greeting = "Hi " plus name
```

In `say` and `print`, adjacent parts are joined in order:

```text
set total to 14
say "Total: " total
```

See [Lists, text, and numbers](lists.md) for `upper`, `lower`, `trim`, `join`, and `split`.
