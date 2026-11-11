# Getting started on Windows

After [installing Romit](../install.md), open a new terminal so `PATH` updates apply:

```powershell
romit --version
```

Create `hello.rmt` with your editor:

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

From PowerShell in that folder:

```powershell
romit check hello.rmt
romit test hello.rmt
```

Clone the repo to run the same file from examples:

```powershell
git clone https://github.com/romit07/romitlang.git
cd romitlang
romit test examples\beginner\hello_world.rmt
```

The CLI commands today are `romit check` and `romit test` — there is no separate `romit run` subcommand yet.
