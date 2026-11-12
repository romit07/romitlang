# Lists, text, and numbers

Romit scripts have lists, decimals, and text building, all without brackets.

## Text

`+` or `plus` joins text, and `{…}` inside a string inserts any value:

```text
name = "Ada"
say "Hello, " + name
say "Hi {name}, you have {count of tasks} tasks"
```

Write `{{` or `}}` for a literal brace. (Braces are only allowed inside text. Code structure still uses indentation.)

## Numbers

Whole numbers stay whole, and anything else becomes a decimal:

```text
say 7 / 2        # 3.5
say 6 / 2        # 3
say 0.1 + 0.2    # 0.3
say round 3.14159, 2
```

## Lists

```text
groceries = list "milk", "eggs"
add "bread" to groceries
remove "eggs" from groceries
empty = list

for each item in groceries
  say "- {item}"

say count of groceries
say first of groceries
say item 2 of groceries          # positions start at 1
if groceries contains "milk"
  say "got milk"
say join groceries, ", "
```

## Built-in functions

Call them like any function (`sort nums`) or with `of` (`sum of nums`).

| Function | Does |
|----------|------|
| `list a, b, c` | make a list |
| `count`, `size`, `length` | items in a list, or letters in text |
| `first`, `last`, `item N of` | pick items |
| `sum`, `average`, `min`, `max` | math over a list |
| `sort`, `reverse`, `unique` | new reordered or de-duplicated list |
| `join list, sep`, `split text, sep` | between lists and text |
| `upper`, `lower`, `trim`, `text`, `number` | text helpers |
| `round x, places`, `floor`, `ceil`, `abs`, `sqrt` | number helpers |
| `range a, b`, `random a, b` | number ranges |

## Changing values

```text
score = 0
score += 10
score -= 2
increase score by 5
decrease lives
```

## Loops

`stop` leaves a loop early and `skip` jumps to the next pass:

```text
for each n in 1 to 100
  if n remainder 2 is 0
    skip
  if n is above 9
    stop
  say n
```

## More

Records, maps, `where` / `each` / `grouped by`, `match`, `try`, `ask`, enums, dates and `use` are covered in [Records, match, and errors](records-match-errors.md).

## Comments

```text
# a comment
note also a comment
say "hi"   # comments can end a line too
```
