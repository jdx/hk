# `hk config`

- **Usage**: `hk config <SUBCOMMAND>`
- **Aliases**: `cfg`

Configuration introspection and management

View and inspect hk's configuration from all sources. Configuration is merged from multiple sources in precedence order: CLI flags > Environment variables > Git config (local) > User config (.hkrc.pkl) > Git config (global) > Project config (hk.pkl) > Built-in defaults.

## Subcommands

- [`hk config dump [--format <FORMAT>]`](/cli/config/dump.md)
- [`hk config explain <KEY>`](/cli/config/explain.md)
- [`hk config get <KEY>`](/cli/config/get.md)
- [`hk config sources`](/cli/config/sources.md)
