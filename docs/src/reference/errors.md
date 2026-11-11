# Error codes

## Parser (P0xx)

| Code | Summary |
|------|---------|
| P001 | Tabs in indentation |
| P002 | Indent not a multiple of two spaces |
| P003 | Dedent beyond outermost block |
| P004 | Indent does not match a prior level |
| P013 | Braces, brackets, parens, or `:` |
| P010 | Unexpected `=` or character (use `set x to value` for assignment) |
| P020 | Expected indented block or statement |
| P021 | Expected a word |

## Type and rules (E1xx, E2xx)

| Code | Summary |
|------|---------|
| E005 | Script mixed with spec declarations |
| E006 | Empty script |
| E107 | `say` or `print` value not printable |
| E108 | Undefined script variable |
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
