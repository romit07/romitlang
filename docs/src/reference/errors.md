# Error codes

## Parser (P0xx)

| Code | Summary |
|------|---------|
| P001 | Tabs in indentation |
| P002 | Indent not a multiple of two spaces |
| P003 | Dedent beyond outermost block |
| P004 | Indent does not match a prior level |
| P010 | Unexpected `=` in an expression (use `is` or `==`); or unexpected character |
| P013 | Braces, brackets, or parentheses (Romit script surface is brace-free) |
| P014 | Trailing comma in a parameter or argument list |
| P015 | Empty slot between commas (`,,`) |
| P020 | Expected indented block or statement |
| P021 | Expected a word |
| P027 | Call syntax used `:` after the name (`fn`/`function` and `:` are for definitions only) |
| P028 | Nested call not allowed except as the last argument on the line |

## Type and rules (E1xx, E2xx)

| Code | Summary |
|------|---------|
| E005 | Script mixed with spec declarations |
| E006 | Empty script |
| E107 | `say` or `print` value not printable |
| E108 | Undefined script variable |
| E110 | `give` used outside a function body |
| E111 | Parameter shadows a variable in an outer scope |
| E112 | Duplicate parameter name in one function |
| E113 | Function can finish without `give` on some path |
| E114 | Wrong number of arguments at a call |
| E115 | Undefined function in a script |
| E116 | Unknown named argument at a call |
| E118 | `fn` / `function` definition missing an indented body |
| E119 | `fn` / `function` missing the function name |
| E199 | Test expectation failed at runtime |
| E210 | Type or field error (checker) |
| E402 | Rule never exercised by tests (coverage) |

## UI framework (U0xx)

| Code | Summary |
|------|---------|
| U001 | Unknown UI part (lists valid parts) |
| U002 | UI file must use `use ui` or valid `use template` |
| U003 | Page declaration missing or duplicate |
| U006 | Unknown theme token (lists valid tokens) |
| U007 | Custom template file invalid or unreadable |
| U008 | Cannot write build output |

## Warnings (W4xx)

| Code | Summary |
|------|---------|
| W410 | Suspicious but legal construct |

Negative fixtures live under `examples/negative/`.
