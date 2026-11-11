# Getting started on Linux

After [installing Romit](../install.md):

```bash
romit --version
```

Create `hello.rmt`:

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

Check and test:

```bash
romit check hello.rmt
romit test hello.rmt
```

Try the bundled beginner examples:

```bash
git clone https://github.com/romit07/romitlang.git
cd romitlang
romit test examples/beginner/hello_world.rmt
```

Official Linux releases target **glibc** GNU triples. Very old distributions may need to build from source with a current Rust toolchain.
