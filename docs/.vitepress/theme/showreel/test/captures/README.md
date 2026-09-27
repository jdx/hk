# Terminal captures

The showreel redraws real hk output. `kit/screens.ts` holds the lines the reel
shows, and `screens.test.ts` checks them against these files: every string is
verbatim, `header()` reproduces every header line, and every character's
colour matches the ANSI that hk printed.

| File | What it is |
| --- | --- |
| `commit.{frames,screen}.txt` | `git commit -m 'feat: hoist the sails'` through the installed pre-commit hook |
| `check-all.{frames,screen}.txt` | `hk check --all` on the repository after that commit |
| `blocked.{frames,screen}.txt` | `git commit -m 'chore: batten the hatches'` with `unused=1` staged in `scripts/deploy.sh`: shellcheck fails, hk exits 1, and git makes no commit |
| `fix.{frames,screen}.txt` | `hk fix` in a fresh copy with the same problems unstaged |
| `styles.txt` | every distinct line the four runs printed, with its SGR escapes written as the two characters `\e` |
| `log.txt`, `log-after-blocked.txt` | `git log --oneline` after the commit, and after the blocked attempt |
| `status-before.txt`, `status-after.txt`, `status-after-blocked.txt` | `git status --short` before the commit, after it, and after the blocked attempt |

A `.frames.txt` file holds the screen at the end of each of hk's redraws. hk
wraps each redraw in synchronized output, and a frame is taken at each
`ESC[?2026l`. Frames are numbered from 0, and `kit/screens.ts` keeps those
numbers. A `.screen.txt` file is the final screen with its scrollback. Lines
are right-trimmed.

## How they were made

- **hk:** a debug build of this repository at commit `10b49864`, which
  reports version `2.2.0`. The reel shows that version (`VERSION` in
  `kit/screens.ts`) rather than the one in `Cargo.toml`, so a release does
  not need a new video.
- **Tools:** prettier 3.9.8 on Node 26.10.0, ruff 0.16.8, shfmt 3.14.1,
  shellcheck 0.11.0 and git 2.53.0.
- **Terminal:** each command ran under util-linux `script -q -e -c "stty cols 80 rows 50; <command>"`
  with `TERM=xterm-256color`, so the terminal was 80 columns wide. The raw
  output was fed through the pyte 0.8.2 VT emulator, at 80 columns and 60
  rows, after removing the header and trailer lines that `script` adds.
- **Environment:** `GIT_CONFIG_NOSYSTEM=1`. The author and committer were
  `Demo <demo@example.com>`. `HK_JOBS`, `HK_STASH`, `HK_STAGE`, `HK_PROFILE`
  and `HK_MISE` were unset. `HOME` was set to the short neutral path
  `/dev/shm/hk`, so the `See …/output.log` line at the end of `blocked` gives
  no machine path. The reel never shows that line.
- **Pinned dates:** `GIT_AUTHOR_DATE` and `GIT_COMMITTER_DATE` were set to
  1790423876 for the initial commit and 1790423879 for the hoist commit, so
  the hashes are always `f92f487` and `ada2ca4`. The blocked attempt ran at
  1790423899.

The demo repository's `hk.pkl` has seven steps, and `ruff-format` depends on
`ruff`:

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.2.0/hk@2.2.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.2.0/hk@2.2.0#/Builtins.pkl"

steps {
  ["prettier"] = Builtins.prettier
  ["ruff"] = Builtins.ruff
  ["ruff-format"] = (Builtins.ruff_format) {
    depends = "ruff"
  }
  ["shfmt"] = Builtins.shfmt
  ["shellcheck"] = Builtins.shellcheck
  ["trailing-whitespace"] = Builtins.trailing_whitespace
  ["newlines"] = Builtins.newlines
}
```

The first commit adds clean `README.md`, `src/app.ts`, `src/main.py` and
`scripts/deploy.sh`. The files were then rewritten with formatting problems
and staged: trailing spaces in `README.md`, unformatted TypeScript, an unused
`import os` and bad spacing in Python, and an over-indented `echo` in the
shell script. Then `# TODO: splice the mainbrace` was appended to
`src/main.py` without staging it, and `hk install` installed the hooks. That
unstaged line is what hk stashes and restores.

## Timing

A run's frames depend on how its steps' timing falls, so a repeat can show steps
starting and finishing in a different order. The storyboard was written
against one run. Its `blocked` capture ended with a scratch path in the
`output.log` line, so that capture was made again with the neutral `HOME`.
The capture was repeated until its frames matched the first run's frames line
for line, except for that path. The third try matched, and its `blocked`
capture is the one here. The other captures come from the first run. Both runs
agree on every git log and status file.

## Raw captures

The raw `script` output is not committed. Its carriage returns and cursor
movements fail the repository's trailing-whitespace check. `styles.txt` keeps
what the test needs from it: each distinct printed line, with the carriage
returns and the cursor, clear and synchronized-output sequences removed, and
the colour codes kept.
