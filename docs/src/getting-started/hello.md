# Hello world

```text
spec HelloWorld version 1

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

Run:

```bash
romit test examples/beginner/hello_world.rmt
```
