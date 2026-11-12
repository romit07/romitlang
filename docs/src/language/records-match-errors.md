# Records, match, and errors

These features cover what Kotlin programs use most: data classes, maps, `when`, `try`/`catch`, enums, and reading input. They work in scripts and in apps.

## Records (data classes)

```text
ada = with name "Ada", age 36
say ada.name
ada.age += 1
set ada.city to "London"          # add or change a field
say ada                            # (name: "Ada", age: 37, city: "London")
```

Records compare by their fields: `with x 1, y 2` is the same as `with y 2, x 1`. They're values, so `copy = ada` makes an independent copy.

A list of records can be written on one line:

```text
people = list with name "Ada", age 36, with name "Alan", age 41
```

## Maps

```text
prices = map "tea" to 3, "green tea" to 4
say item "green tea" of prices
set item "cake" of prices to 5
say keys of prices                 # tea, green tea, cake
say values of prices
if prices contains "cake"
  say "we have cake"
```

## Nothing and defaults

`nothing` means "no value", like Kotlin's `null`. `or else` gives a default, like Kotlin's `?:`:

```text
nickname = nothing
say "Hi {nickname or else "friend"}"
if nickname is nothing
  say "no nickname yet"
```

Reading a field that doesn't exist gives `nothing` instead of crashing.

## Working with lists

| Write | Kotlin |
|-------|--------|
| `people where age is above 30` | `filter { it.age > 30 }` |
| `people each name` | `map { it.name }` |
| `nums each it times 2` | `map { it * 2 }` |
| `people sorted by age`, `… sorted by age descending` | `sortedBy`, `sortedByDescending` |
| `people grouped by team` | `groupBy { it.team }` (a map of lists) |
| `first 3 of xs`, `last 2 of xs` | `take(3)`, `takeLast(2)` |
| `items 2 to 4 of xs` | `slice(1..3)` (Romit counts from 1) |
| `count of people where age is above 30` | `count { … }` |

Inside `where` and `each`, `it` is the current item and its fields are available by name (`age`, or `it.age`). An item's own field wins over an outer variable with the same name.

`first 3 of todos where done` takes the first three *done* todos: filters and sorting belong to the list. `item t of teams each name` gives the names in team `t`: `each` and `grouped by` apply to the picked result.

## Text

| Write | Gives |
|-------|-------|
| `replace "cat" with "dog" in story` | text with every match replaced |
| `name starts with "A"`, `file ends with ".txt"` | yes / no |
| `letters 1 to 3 of name`, `first 2 of name` | part of the text |
| `"Tab\there"`, `"Say \"hi\""` | escapes: `\n`, `\t`, `\"`, `\\` |
| `"Hi {name or else "friend"}"` | quotes are fine inside `{…}` |

Multi-line text uses three quotes. Shared indentation is removed:

```text
poem = """
  Roses are red,
  Romit is {upper "new"}.
  """
```

## match (Kotlin's `when`)

```text
match size
  "S"
    say "small"
  "M", "L"
    say "medium or large"
  otherwise
    say "something else"

match temperature
  below 0
    say "freezing"
  is at most 20
    say "mild"
  otherwise
    say "hot"

match                      # no subject: the first true condition wins
  score is at least 90
    say "A"
  otherwise
    say "keep going"
```

`match` works in scripts, app actions, and app views (where it chooses what to show).

## Errors: try, if it fails, fail

```text
fn withdraw balance, amount
  if amount is above balance
    fail "not enough money"
  give balance minus amount

try
  say withdraw 100, 250
if it fails
  say "Sorry, {error}"
```

`fail "message"` stops with an error (Kotlin's `throw`). `try` … `if it fails` catches it, and `error` holds the message. An error nobody catches stops the program and shows the message (code **E901**).

In apps, failures from `load "https://…"` and `open file` can be caught the same way. Without `try`, the person sees a short message.

## Enums

```text
enum Size small, medium, large
say Size                   # small, medium, large (like Kotlin's entries)
mine = medium
say mine is in Size
```

Each value is a constant with its own name as text. In apps, `picker bind pick options Size` shows them as choices.

## Asking questions

```text
name = ask "What's your name?"
age = ask "How old are you?"
say "Next year you'll be {age + 1}, {name}."
```

`ask` waits for a line of input. Numbers come back as numbers, unless they don't read back the same way (`"007"` stays text). With no input left, it gives `nothing`. In the playground, type answers in **Input for ask**, one per line.

## Several files

```text
use "helpers.rmt"
say shout "hello"
```

`use` brings in another file's functions and top-level values. The path is relative to the file that uses it. Apps can `use` files with shared `view`s, `action`s, screens and state. Functions can read top-level values, like Kotlin's top-level `val`s.

## Dates

Scripts write dates as text like `"2026-11-12"` (UTC).

```text
say today                                  # 2026-11-12
say now                                    # 2026-11-12 18:40
say weekday of "2024-02-29"                # Thursday
say days from "2024-01-01" to today
say year of today
```

Apps also have `as date`, `as time`, `as ago`, and a date picker.

## Loops that count down or skip

```text
for each i in 10 to 1          # counts down
for each i in 0 to 100 by 25   # 0, 25, 50, 75, 100
```
