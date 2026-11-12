# Kotlin → Romit

This page goes through [Kotlin's documentation](https://kotlinlang.org/docs/home.html) topic by topic and shows how to do each thing in Romit, or says plainly that Romit doesn't.

Romit isn't trying to be Kotlin. It has no classes, generics or static types, because those work against "write it like a prompt". It does aim to cover what everyday programs need, and its app layer overlaps with **Compose Multiplatform**.

**Status:** ✅ supported · 🟡 partly or another way · ❌ missing · ➖ doesn't apply to Romit's design

## Basics and types

| Kotlin | Romit | |
|---|---|---|
| `val` / `var` | `x = 5`, `set x to 5` (no read-only values) | 🟡 |
| Operators | symbols and words (`+` / `plus`, `>=` / `is at least`) | ✅ |
| String templates `"$x ${a.b}"` | `"{x} {a.b}"`, any expression inside, quotes allowed | ✅ |
| Raw and multi-line strings | `"""…"""` (indentation trimmed), escapes `\n \t \" \\` | ✅ |
| String functions | `upper`, `lower`, `trim`, `split`, `join`, `replace … with … in`, `starts with`, `ends with`, `contains`, `letters 1 to 3 of`, `length` | ✅ |
| Numbers | whole numbers and decimals, `7 / 2` = 3.5, `1_000`; no Byte/Float/hex literals | 🟡 |
| Booleans | `true` / `false` | ✅ |
| Characters | single letters are text (`item 2 of "hi"`) | 🟡 |
| Arrays and lists | `list a, b`, `add`, `remove`, `item n of`, `first/last N of`, `items a to b of` | ✅ |
| Maps | `map "a" to 1`, `item k of m`, `set item k of m to v`, `keys of`, `values of`, `contains` | ✅ |
| Sets | `unique of xs` | 🟡 |
| Ranges | `1 to 5`, `10 to 1` (down), `by 2` (step), `range 1, 5` | ✅ |
| Null safety (`?`, `?:`, `?.`) | `nothing`, `is nothing`, `or else`; a missing field gives `nothing` | ✅ |
| Type checks and casts | no static types; `number of`, `text of` convert | ➖ |
| Unsigned types, type aliases | — | ➖ |
| Packages and imports | `use "file.rmt"` (scripts and apps) | ✅ |
| Annotations, visibility modifiers | — | ➖ |

## Control flow and functions

| Kotlin | Romit | |
|---|---|---|
| `if` / `else` (also as an expression) | `if` / `otherwise if` / `otherwise`; `if … then … else …` | ✅ |
| `when` | `match` with values, comparisons, or no subject | ✅ |
| `for`, `while` | `for each x in xs`, `for each i in 1 to 9 by 2`, `while` | ✅ |
| `do-while` | `while` with `stop` | 🟡 |
| `return`, `break`, `continue` | `give`, `stop`, `skip` (no labels) | ✅ |
| Exceptions: `throw`, `try` / `catch` | `fail "…"`, `try` … `if it fails` (`error` holds the message) | ✅ |
| `finally` | — | ❌ |
| Functions, defaults, named arguments, recursion | `fn name a, b`, `b is 2`, `with b 3`, recursion | ✅ |
| Top-level properties | functions can read top-level values | ✅ |
| Varargs, local functions, return types | — | ❌ |
| Lambdas and higher-order functions | `where`, `each`, `sorted by` take an expression with `it`; apps pass actions (`do …`, `on tap`); no lambda values | 🟡 |
| Type-safe builders (DSLs) | the app syntax is a builder DSL like Compose | 🟡 |
| Inline functions, operator overloading, context parameters, extensions | — | ➖ |
| Reading input `readln()` | `ask "Question?"` | ✅ |

## Classes and objects

| Kotlin | Romit | |
|---|---|---|
| Data classes | records: `with name "Ada", age 36`, field access and change, structural equality, readable printing | ✅ |
| Destructuring | — | ❌ |
| Enum classes, `entries` | `enum Size small, medium, large`; `Size` lists the values | ✅ |
| Sealed classes | spec `machine` (allowed state changes checked by the compiler) | 🟡 |
| `object` (singletons) | an app's `state` is one shared object | 🟡 |
| Properties, getters, observers | app `state`, `computed`, `on change of x`, `saved` (persisted) | 🟡 |
| Classes, inheritance, interfaces, generics, delegation | — | ➖ |

## Standard library

| Kotlin | Romit | |
|---|---|---|
| Filter, map, sort, group | `where`, `each`, `sorted by … descending`, `grouped by` | ✅ |
| Aggregate | `count`, `sum`, `average`, `min`, `max` | ✅ |
| Take, drop, slice, first, last | `first N of`, `last N of`, `items a to b of`, `first of`, `last of` | ✅ |
| `zip`, `flatMap`, `windowed` | — | ❌ |
| Scope functions, sequences, reflection | — | ➖ |
| Date and time | `today`, `now`, `year` / `month` / `day` / `weekday of`, `days from a to b`; apps: `as date`, `as ago`, date picker | 🟡 |
| JSON (kotlinx.serialization) | automatic: `saved`, `load "url" into x`, `open file … as json`, `save x to file` | 🟡 |
| HTTP client (Ktor client) | `load "https://…" into x` (GET) | 🟡 |
| Coroutines | apps: async actions (`wait`, `load`), timers (`every`), reactive state; no channels or flows | 🟡 |
| Testing (kotlin.test) | spec `test` blocks with `romit test` | 🟡 |
| UUIDs, HTTP server, databases | — | ❌ |

## Platforms and tools

| Kotlin | Romit | |
|---|---|---|
| **Compose Multiplatform + Material 3** | **Romit apps**: Material 3 on Android, Apple style on iPhone | ✅ |
| Kotlin/JS, web | apps compile to one HTML file | ✅ |
| Android, iOS | apps run in the phone's browser with a native look; no store builds | 🟡 |
| JVM, Native, Wasm targets, Java/C/Swift interop | — | ❌ |
| Scripting (`.kts`) | `romit run script.rmt` | ✅ |
| IDE support | VS Code extension: highlighting, errors as you type, autocomplete, snippets, run command | 🟡 |
| Gradle / Maven | one `romit` tool | ➖ |

See also: [SwiftUI → Romit](swiftui-map.md), [Records, match, and errors](../language/records-match-errors.md).
