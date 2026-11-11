# Getting started on macOS

After [installing Romit](../install.md):

```bash
romit --version
```

Create `hello.rmt` in a new folder:

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

Type-check it:

```bash
romit check hello.rmt
```

Run tests in the file:

```bash
romit test hello.rmt
```

The repository example is the same program:

```bash
romit test examples/beginner/hello_world.rmt
```

Use `romit check --no-coverage <file>` on large specs when you want to skip rule-coverage diagnostics.
