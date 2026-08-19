# usage 6.x migration status

This branch converts hk's real typed CLI from clap to usage-rs and removes clap
from the runtime dependencies. `cargo check` succeeds and all 254 binary tests
pass after generating the repository's expected Pkl artifacts. It is not
intended to merge before usage 6.x because it deliberately pins a stacked git
revision.

The working port still records the compatibility gaps it had to handle locally:

- positional-file relationships are enforced after binding because the spec
  cannot attach conflicts to a positional argument;
- `run` retains clap's `arg_required_else_help` behavior with an explicit
  no-hook error until usage supports that command policy;
- command effects require parsing and mutating the derived KDL through
  usage-lib, whose MSRV is newer than hk's declared Rust version;
- clap-only attribute spellings such as `num_args` and array value parsers need
  semantic usage declarations rather than a textual derive rename.
