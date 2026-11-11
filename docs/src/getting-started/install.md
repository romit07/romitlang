# Install and first check

```bash
cargo build --release
./target/release/romit check examples/beginner/hello_world.rmt
./target/release/romit test examples/beginner/hello_world.rmt
```

The CLI commands are `romit check <file.rmt>` and `romit test <file.rmt>`. Use `romit check --no-coverage <file>` to skip rule-coverage errors (**E402**) on large specs.
