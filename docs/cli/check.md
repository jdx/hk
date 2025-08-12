# `hk check`

- **Usage**: `hk check [FLAGS] [FILES]…`
- **Aliases**: `c`

Fixes code

## Arguments

### `[FILES]…`

Run on specific files

## Flags

### `-a --all`

Run on all files instead of just staged files

### `-f --fix`

Run fix command instead of run command This is the default behavior unless HK_FIX=0

### `-c --check`

Run run command instead of fix command

### `-e --exclude… <EXCLUDE>`

Exclude files that otherwise would have been selected

### `--exclude-glob… <EXCLUDE_GLOB>`

Exclude files that match these glob patterns that otherwise would have been selected

### `--from-ref <FROM_REF>`

Start reference for checking files (requires --to-ref)

### `--to-ref <TO_REF>`

End reference for checking files (requires --from-ref)

### `-g --glob… <GLOB>`

Run on files that match these glob patterns

### `-P --plan`

Print the plan instead of running the hook

### `-S --step… <STEP>`

Run specific step(s)

## Timing report (JSON)

To capture wall-time spent during a run, set `HK_TIMING_JSON` to a file path. This writes a JSON report when the run completes, including total wall time and per-step wall time (overlaps are merged, so parallel parts aren’t double-counted).

```bash
HK_TIMING_JSON=/tmp/hk-timing.json hk check
cat /tmp/hk-timing.json | jq
```
