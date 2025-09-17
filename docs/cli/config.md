# `hk config`

- **Usage**: `hk config <SUBCOMMAND>`
- **Aliases**: `cfg`

Configuration introspection and management

View and inspect hk's configuration from all sources. Configuration is merged from multiple sources in precedence order: CLI flags > Environment variables > Git config (local) > Git config (global/system) > User config (.hkrc.pkl) > Project config (hk.pkl) > Built-in defaults.

## Subcommands

- [`hk config dump [--format <FORMAT>]`](/cli/config/dump.md)
- [`hk config get <KEY>`](/cli/config/get.md)
- [`hk config show`](/cli/config/show.md)
- [`hk config sources`](/cli/config/sources.md)
