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
| P029 | `{` in text without a matching `}` (use `{{` for a literal brace) |

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

### Apps (`app` / `screen`)

| Code | Summary |
|------|---------|
| U001 | Unknown element (with "did you mean") |
| U003 | App structure: missing screen, bad screen or view line, block in the wrong place |
| U006 | Unknown theme setting |
| U101 | Unknown name: not a state value, computed value, loop item, or parameter |
| U102 | Bad expression or unexpected word on a line |
| U103 | Bad `state` or `computed` line (`name is value`) |
| U104 | Unknown event, or event in the wrong place (`every` and `on change of` go on screens or the app) |
| U105 | Modifier not indented under an element |
| U106 | Bad `for each … in …` |
| U107 | Modifier is missing its value |
| U108 | Bad time for `every` |
| U109 | Unknown or malformed action |
| U110 | Name can't be used for state (modifier or reserved word) |
| U111 | Name or screen defined twice |
| U112 | Changing something that can't change (computed values, loop items) |
| U113 | Unknown screen, or wrong number of values passed to it |
| U114 | Unknown action, or wrong number of values passed to it |
| U115 | Unknown view, or wrong number of values passed to it |
| U116 | Control is missing `bind`, `options`, or `to` |
| U117 | State value uses another one before it is defined |
| U118 | `document` problem: no `document` line, or it names something not in `state` |

## Warnings (W4xx)

| Code | Summary |
|------|---------|
| W410 | Suspicious but legal construct |

Negative fixtures live under `examples/negative/`.
