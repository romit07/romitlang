# Install and first check

This page moved to the dedicated [Install](../install.md) chapter.

Quick path:

```bash
curl -fsSL https://romitlang.org/install.sh | sh
romit test examples/beginner/hello_world.rmt
```

The CLI commands are `romit check <file.rmt>` and `romit test <file.rmt>`. Use `romit check --no-coverage <file>` to skip rule-coverage errors (**E402**) on large specs.
