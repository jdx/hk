# usage 6.x adoption experiment

This generated crate proves that usage's static tables can express hk's current
checked-in spec: 54 commands, 357 flags, and 44 positional arguments are emitted
without dropping a spec field. It intentionally pins usage to a git revision and
is not intended to merge before usage 6.x.

It is not yet the typed hk CLI. Converting the real clap structs still has to
resolve:

- relationship families such as `conflicts_with_all` and `overrides_with_all`;
- `num_args`, including optional-value flags with default-missing values;
- custom and array value parsers;
- expression-valued defaults such as generated settings constants;
- clap value enums, value hints, and verbatim doc-comment behavior;
- the nested command/flatten graph and dynamic version metadata.

The generated source is a concrete compile target and baseline for the typed
conversion. Regenerate it from the usage repository with:

```console
cargo run -p xtask -- gen-shadow /path/to/hk/hk.usage.kdl /path/to/hk/experiments/usage6 usage
```

