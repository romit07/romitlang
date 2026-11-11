# Strings and interpolation

String interpolation does not use `{` or `}`. Use **adjacent words** after a string literal (the value is concatenated in order):

```text
note documented join form (v0.3):
say "Hello" name
```

equivalent to building the message from `"Hello "` and the value of `name`. The longer spelling is:

```text
"Hello " join name
```

Only one of these forms is required in source; this book uses `say "Hello" name` in examples.
