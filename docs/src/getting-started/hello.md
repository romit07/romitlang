# Hello world

Create `hello.rmt`:

```text
say "Hello, world!"
```

Run it:

```bash
romit run hello.rmt
```

You should see:

```text
Hello, world!
```

## Spec form (optional)

The same greeting as a full spec with a test lives in `examples/beginner/hello_world.rmt`:

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

```bash
romit test examples/beginner/hello_world.rmt
```

Use scripts for quick tools; use specs when you need rules, invariants, and tests.
