# Swift feature map

Inspiration from [The Swift Programming Language](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/). Romit aims for practical coverage with shorter surface syntax; **proposal** unless marked **shipped**. Batches follow the [roadmap](roadmap.md).

| Swift area | Swift (summary) | Romit (proposal) | Rationale | Status | Batch |
|-----------|---------------|------------------|-----------|--------|-------|
| Basics | `let` / `var`, `+=` | `set x to 5`, `x = 5`, `x += 1`, `increase x by 2`, decimals (`7 / 2` is 3.5) | Plain assignment; compiler infers flow | shipped | 1–2 |
| Operators | `+ - * / %`, `==`… | symbols **and** `plus`, `is at least`, … | Same meaning; words optional | shipped | 1 |
| Functions | `func f(_ a: Int) -> Int` | `sum: a, b` / `fn sum a, b` body `give` | Colon header + indentation; `fn`/`function` aliases | shipped | 1 |
| Calls | `f(1, 2)` | `f 1, 2` (no parens) | Space/comma arg lists | shipped | 1 |
| Default params | `name: String = "x"` | `fn greet name is "world"` | Default after `is` | shipped | 1 |
| Control flow | `if` / `else` | `if` / `otherwise if` / `otherwise` blocks | Indentation blocks, no braces | shipped | 1 |
| Loops | `for-in`, `while`, `break`, `continue` | `for each i in 1 to 5`, `for each x in xs`, `while`, `repeat N times`, `stop`, `skip` | Plain words | shipped | 1–2 |
| Optionals | `T?`, `if let` | (planned) optional words | Shorter chaining later | planned | 3 |
| Strings | `"\(x)"` | `"Hi {name}"`, `+` / `plus`, `say "Hi " name` | Interpolation inside text only; code stays brace-free | shipped | 2 |
| Collections | Array, Set, Dict | `list 1, 2, 3`, `add x to xs`, `for each x in xs`, `count of xs`; records in UI apps (`with title "a", done false`) | No brackets | partial | 2 |
| Structs / classes | `struct`, `class` | (planned) `type` blocks | Value types first | planned | 4 |
| Enums | `enum` associated values | spec `enum` + script enums later | Spec layer today | partial | 3 |
| Error handling | `throw` / `try` | (planned) `reject` / `recover` | Align with spec `reject` | planned | 4 |
| Generics | `<T>` | (planned) type parameters on `fn` | No angle brackets | planned | 5 |
| Protocols | `protocol` | (planned) `shape` / requirements | Word-based constraints | planned | 5 |
| Extensions | `extension` | (planned) `extend` | Same semantics, shorter | planned | 5 |
| Access control | `public` / `private` | (planned) `shared` / `file` | Shorter labels | planned | 6 |
| Concurrency | `async`/`await` | (planned) `later` / `wait` | Batch after core runtime | planned | 6 |
| Pattern matching | `switch` / `case` | `if` / `otherwise` + future `match` | Batch 1 if/otherwise | partial | 1 |
| Property wrappers | `@Wrapper` | (planned) | Not in scope yet | planned | 7 |
| Macros | `#macro` | (planned) | Not in scope yet | planned | 7 |

For **SwiftUI** (views, layout, state, navigation, modifiers), see [SwiftUI → Romit](swiftui-map.md).

## Batch 2

**Shipped:** text interpolation (`"Hi {name}"`), `+` joins text, decimals and true division, `#` / `note` comments, `+=` / `-=` / `increase` / `decrease`, lists with `add` / `remove` / `for each … in` and built-ins (`count`, `sum`, `sort`, `join`, …), `stop` / `skip`, and Romit apps (`use ui` + `app` / `screen`).

## Batch 1

**Shipped in the compiler and docs:** script functions (`sum:`, `fn`, `function`), `give`, comma/space parameters and call arguments, control-flow statements, operator aliases (`+` / `plus`, `is at least` / `>=`, `%` / `remainder`), validation codes through **E119** / **P028**, playground and beginner examples.

**Deferred to later batches:** entities and collections in the script interpreter, optionals, native compilation, full generic and protocol syntax.
