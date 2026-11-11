# Hello world

Create `hello.rmt`:

```text
say "Hello, world!"
```

`print` works the same way (`print "Hello, world!"`). Romit keeps **`say`** as the preferred short form; **`print`** is an alias for beginners coming from Python or Lua.

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

Use `say` or `print` scripts for small programs; add a `spec` only when you want optional checks, invariants, and tests.

```bash
romit run examples/beginner/print_alias.rmt
```

## Variables in scripts

Assign with **`x = 5`** (short form) or **`set x to 5`** (readable form). Both mean the same thing; spacing after `=` is optional (`x=5` works too).

```text
total = 3 times 4 plus 2
say "Total: " total
x = 5
x = x plus 1
say x
```

The wordy form is still fine:

```text
set total to 3 times 4 plus 2
set x to 5
```

```bash
romit run examples/beginner/assign_equals.rmt
```

Expected output:

```text
Hi Ada
Total: 14
6
```

For a landing page, see [UI framework](../ui-framework.md) and `examples/ui/landing.rmt` — the compiler fills in layout and theme defaults.
