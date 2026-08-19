# usage 6.x migration status

This branch converts hk's real CLI structs and attributes from clap to usage-rs
and removes clap from the runtime dependencies. It is an experimental PR and is
not intended to merge before usage 6.x.

`cargo check` currently fails after reaching the usage derives. The 490
diagnostics include cascades from these underlying gaps:

- usage command variants must wrap a dedicated Args struct, while hk has inline
  variants and shares command argument types;
- `HookOptions` is flattened into many hook commands, but a single Args type
  cannot currently collect for several commands;
- fixed and optional `num_args`, custom/array value parsers, expression-valued
  defaults, aggregate conflicts/overrides, and command-level aliases do not map
  losslessly to usage attributes;
- hk's generated settings and dynamic version metadata are Rust expressions,
  while the corresponding usage attributes require literals;
- clap's `CommandFactory` path used for generated specs and sorting has no
  direct replacement at the existing call sites.

The branch keeps the real conversion and its compiler failures visible. A
parallel String shadow would compile, but would not exercise these typed CLI
constraints.
