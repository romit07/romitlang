# Operators

Romit accepts **symbols** and **words** for arithmetic and comparisons everywhere in scripts (including function bodies, conditions, and the playground). Parentheses are not available for grouping—use a temporary variable when precedence is not what you want.

## Arithmetic

| Meaning | Symbols | Words |
|---------|---------|-------|
| add | `+` | `plus` |
| subtract | `-` | `minus` |
| multiply | `*` | `times` |
| divide | `/` | `divided by` |
| remainder | `%` | `remainder` |

Examples:

```romit
set x to a+b
set y to a plus b
set m to 10 remainder 3
```

Precedence is familiar: `*` / `%` bind tighter than `+` / `-`. Without parentheses, split sub-expressions into variables:

```romit
set step1 to a plus b
set step2 to step1 times c
```

## Comparisons

| Symbols | Words |
|---------|-------|
| `==` | `is` |
| `!=` | `is not` |
| `<` | `is below`, `is less than`, `below` |
| `>` | `is above`, `is more than`, `above` |
| `<=` | `is at most` |
| `>=` | `is at least` |

Examples:

```romit
if n is below 2
  give 1
if score>=80
  say "high"
```

The spec-layer parser accepts the same aliases in handler conditions and tests.
