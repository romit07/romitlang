# Hello world

Create `hello.rmt`:

```text
say "Hello, world!"
```

`print` works the same way (`print "Hello, world!"`). Romit keeps **`say`** as the shortest preferred form; **`print`** is an alias for beginners coming from Python or Lua.

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

Use `say` or `print` scripts for the shortest programs; add a `spec` only when you want optional checks, invariants, and tests.

```bash
romit run examples/beginner/print_alias.rmt
```

## Variables in scripts

Assign with plain words (not `=`):

```text
set total to 3 times 4 plus 2
say "Total: " total
set x to 5
set x to x plus 1
say x
```

```bash
romit run examples/beginner/variables.rmt
```

Expected output:

```text
Total: 14
6
```
