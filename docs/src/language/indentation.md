# Blocks and indentation

Each block is introduced by a header line, then one or more lines indented **two spaces** deeper.

```text
spec MySpec version 1

state
  count int is 0

on Tick
  rule bump
    when true
    set count to count plus 1
```

## Validator codes

| Code | Meaning |
|------|---------|
| **P001** | Tab character used for indentation |
| **P002** | Odd number of spaces (use multiples of two) |
| **P003** | Dedent past the outermost block |
| **P004** | Indentation does not match any enclosing block |
| **P020** | Parser expected an indented block |

Tabs are never allowed. Mixed dedent levels must match a previous indent level on the stack.
