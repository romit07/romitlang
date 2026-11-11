# Brace-free surface

Romit source must not use braces, brackets, or parentheses in the surface syntax. If the lexer sees `(`, `)`, `[`, `]`, `{`, `}`, or a stray `:`, it reports **P013**:

> Romit has no braces. Use indentation instead.

Allowed non-word symbols in code are limited to:

- Operators `+ - * / < >` and `==` / `!=` (or words such as `is`, `is not`, `above`, `below`)
- String quotes `"`
- Hex colours `#RRGGBB`
- Comments `//` or `note`

Calls use **space-separated words**, for example `emit Greet with message "Hi"` or `set answer to 3 times 4 plus 2`.

Types and collections use words instead of punctuation: `List of Number`, `items at 0`, `go to /signup`.
