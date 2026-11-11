# Error codes

## Parser (P0xx)

| Code | Summary |
|------|---------|
| P001 | Tabs in indentation |
| P002 | Indent not a multiple of two spaces |
| P003 | Dedent beyond outermost block |
| P004 | Indent does not match a prior level |
| P013 | Braces, brackets, parens, or `:` |
| P020 | Expected indented block |
| P021 | Expected a word |

## Type and rules (E1xx, E2xx)

| Code | Summary |
|------|---------|
| E199 | Test expectation failed at runtime |
| E210 | Type or field error (checker) |
| E402 | Rule never exercised by tests (coverage) |

## Warnings (W4xx)

| Code | Summary |
|------|---------|
| W410 | Suspicious but legal construct |

Negative fixtures live under `examples/negative/`.
