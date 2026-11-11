# Beginner programs

These programs live in the repository under `examples/beginner/`. Run any file with:

```text
romit test examples/beginner/<name>.rmt
```

## hello_world.rmt

```text
spec HelloWorld version 1
about "Say hello — the smallest Romit program."

output Greet
  message text

event Run

on Run
  rule say_it
    when true
    emit Greet with message "Hello, world!"

test says_hello
  send Run
  expect emitted Greet with message "Hello, world!"
```

## calculator.rmt

See the [Calculator tutorial](../tutorials/calculator.md).

```text
spec Calculator version 1
about "Multiply and add in one line — no buttons required."

state
  answer int is 0
event Evaluate

on Evaluate
  rule sample
    when true
    set answer to 3 times 4 plus 2

test two_times_three_plus_two
  send Evaluate
  expect answer is 14
```

## fibonacci.rmt

```text
spec Fibonacci version 1
about "Small Fibonacci answers without recursion — Stage 2 adds real functions."

state
  result int is 0

event Compute with n int

on Compute with n if n is 0
  rule zero
    when true
    set result to 0

on Compute with n if n is 1
  rule one
    when true
    set result to 1

on Compute with n if n is 10
  rule ten
    when true
    set result to 55

test fib_of_10
  send Compute with n 10
  expect result is 55

test fib_of_0
  send Compute with n 0
  expect result is 0

test fib_of_1
  send Compute with n 1
  expect result is 1
```

## todo_list.rmt

Walkthrough: [Todo list tutorial](../tutorials/todo.md).

```text
spec TodoList version 1
about "One task at a time: add text, then mark it done."

state
  task_text text is ""
  task_done bool is false

event Add
event MarkDone

output TaskAdded
  title text

output TaskCompleted
  title text

on Add with title
  rule create
    when not task_done or task_text is ""
    set task_text to title
    set task_done to false
    emit TaskAdded with title title

on MarkDone
  rule finish
    when task_text is not "" and not task_done
    set task_done to true
    emit TaskCompleted with title task_text

test add_then_done
  send Add with title "Buy milk"
  expect task_text is "Buy milk"
  expect emitted TaskAdded with title "Buy milk"
  send MarkDone
  expect task_done is true
  expect emitted TaskCompleted with title "Buy milk"
```

## traffic_light.rmt

Walkthrough: [Traffic light tutorial](../tutorials/traffic.md).

```text
spec TrafficLight version 1
about "A traffic light that cycles red → green → yellow."

enum Color red green yellow

state
  light Color is red

event Tick

output ShowLight
  color text

machine light
  red to green
  green to yellow
  yellow to red

on Tick if light is red
  rule advance
    when true
    set light to green
    emit ShowLight with color green

on Tick if light is green
  rule go_yellow
    when true
    set light to yellow
    emit ShowLight with color yellow

on Tick if light is yellow
  rule go_red
    when true
    set light to red
    emit ShowLight with color red

invariant only_known_colors
  light is red or light is green or light is yellow

test red_to_green
  send Tick
  expect light is green
  expect emitted ShowLight with color green

test full_cycle
  given light is green
  send Tick
  expect light is yellow
  send Tick
  expect light is red
```
