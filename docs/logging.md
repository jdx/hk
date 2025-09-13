---
outline: "deep"
---

# Logging and Debugging

hk provides several ways to control logging output for debugging issues and understanding what's happening during execution.

## Log Levels

hk supports standard log levels that control the amount of information displayed:

- **error**: Only show error messages
- **warn**: Show warnings and errors (default)
- **info**: Show informational messages, warnings, and errors
- **debug**: Show debug information including file operations and step execution details
- **trace**: Show detailed trace information including all internal operations

## Setting Log Levels

### Using CLI Flags

The simplest way to control logging is through command-line flags:

```bash
# Show debug output (includes what files are being checked/fixed)
hk check -v
hk fix --verbose

# Show trace output (very detailed, includes all internal operations)
hk check -vv
```

The `-v` flag can be used multiple times:
- `-v` or `--verbose`: Sets log level to DEBUG
- `-vv`: Sets log level to TRACE

### Using Environment Variables

You can also set the log level using the `HK_LOG` environment variable:

```bash
# Set log level to debug for a single command
HK_LOG=debug hk check

# Set log level to trace
HK_LOG=trace hk fix

# Export for all commands in the session
export HK_LOG=debug
hk check
hk fix
```

### Log File Output

By default, hk writes logs to a file at `~/.local/state/hk/hk.log`. You can control this with environment variables:

```bash
# Change the log file location
HK_LOG_FILE=/tmp/my-hk.log hk check

# Set a different log level for the file (defaults to HK_LOG)
HK_LOG_FILE_LEVEL=trace hk check
```

This is useful when you want minimal console output but detailed file logging for later analysis.

## Tracing and Performance Diagnostics

hk includes built-in tracing support for performance analysis and detailed execution tracking.

### Enabling Tracing

Use the `--trace` flag or `HK_TRACE` environment variable:

```bash
# Enable tracing with console output
hk check --trace

# Enable tracing via environment variable (text mode)
HK_TRACE=1 hk check
HK_TRACE=text hk check

# Enable tracing with JSON output for programmatic analysis
HK_TRACE=json hk check > trace.jsonl
hk check --trace --json > trace.jsonl
```

### Trace Output Formats

**Text Mode**: Human-readable hierarchical output showing spans and timing:
```
  0.123s INFO Starting check
  0.456s ├─ lint::eslint
  0.789s │  ├─ Running eslint on 45 files
  1.234s │  └─ Complete (478ms)
  1.567s └─ Complete (1.444s)
```

**JSON Mode**: Outputs newline-delimited JSON (JSONL) for programmatic analysis:
```json
{"type":"meta","span_schema_version":1,"hk_version":"1.12.1","pid":12345}
{"type":"span_start","ts_ns":123456,"id":"span_0","name":"check","attrs":{}}
{"type":"span_start","ts_ns":456789,"id":"span_1","name":"lint","attrs":{"step":"eslint"},"parent_id":"span_0"}
{"type":"span_end","ts_ns":789012,"id":"span_1"}
{"type":"span_end","ts_ns":1234567,"id":"span_0"}
```

### Performance Timing Reports

Generate JSON timing reports for analysis:

```bash
# Write timing report to a file
HK_TIMING_JSON=/tmp/timing.json hk check

# The report includes:
# - Total wall time
# - Per-step wall time (with overlapping intervals merged)
# - Profile information for each step
```

Example timing report:
```json
{
  "total": { "wall_time_ms": 12456 },
  "steps": {
    "lint": { "wall_time_ms": 4321, "profiles": ["ci", "fast"] },
    "fmt": { "wall_time_ms": 2100 },
    "typecheck": { "wall_time_ms": 6035 }
  }
}
```

## Quiet and Silent Modes

To reduce output:

```bash
# Suppress non-error output
hk check --quiet
hk check -q

# Suppress all output (only exit codes)
hk check --silent
```

## Common Debugging Scenarios

### Debugging Step Execution

To see which files are being processed by each step:

```bash
# Use debug level to see file operations
HK_LOG=debug hk check

# Use trace level for maximum detail
HK_LOG=trace hk check
```

### Debugging Performance Issues

To identify slow steps:

```bash
# Generate a timing report
HK_TIMING_JSON=/tmp/timing.json hk check
cat /tmp/timing.json | jq .

# Use tracing to see detailed timing
hk check --trace
```

### Debugging Configuration Issues

To see how configuration is being loaded and processed:

```bash
# Validate configuration with verbose output
hk validate -v

# Check which steps would run
HK_LOG=debug hk check --dry-run
```

### Debugging Git Hook Issues

When git hooks aren't working as expected:

```bash
# Test hooks directly with debug output
HK_LOG=debug hk run pre-commit

# Check hook installation
hk install --verbose
```

## Tips

1. **Start with `-v`**: For most debugging, `hk check -v` provides enough detail without overwhelming output
2. **Use log files**: Set `HK_LOG_FILE_LEVEL=trace` to capture detailed logs without cluttering the console
3. **Combine with other tools**: Pipe JSON trace output to tools like `jq` for analysis
4. **Profile-specific debugging**: Use `HK_LOG=debug hk check --profile slow` to debug specific profiles
