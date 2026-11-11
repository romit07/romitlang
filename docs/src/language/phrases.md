# Calls and phrases

Romit expresses calls and control flow as **space-separated words** instead of punctuation-heavy syntax.

## Calls and assignments

Emit events and pass fields with phrases:

```text
emit Greet with message "Hi"
set answer to 3 times 4 plus 2
send Compute with n 10
```

Comparisons and arithmetic can use words (`is`, `is not`, `times`, `plus`) or the allowed symbols (`+`, `-`, `*`, `/`, `==`, `!=`).

## Event headers and parameters

Handlers name their event and parameters in the same phrase style:

```text
on Compute with n if n is 10
on Add with title
on Tick if light is red
```

## Types and access

Types read as phrases: `List of Number`, `items at 0`, `task_text is not ""`.

## Navigation and UI (planned)

Planned UI examples use navigation phrases such as `go to /signup` and `on click go to …` instead of arrows or braces. See [UI framework](../ui-framework.md) and [Planned UI syntax](../examples/planned.md).
