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
| `commit.locks.txt` | the commit run's log, cut to its step commands, lock waits and `check_diff` outcomes: the order of the lanes scene's patches and hand-offs |
| `ruff-format.diff.txt` | ruff's diff of `src/main.py` as staged: what the `ruff_format` builtin's `check_diff` prints, which `hk check` shows |
| `ruff-format.after-ruff.diff.txt` | the same command's diff once ruff has removed the unused import: what `hk fix` applies, since `ruff-format` depends on `ruff` |
| `styles.txt` | every distinct line the four runs printed, with its SGR escapes written as the two characters `\e` |
| `log.txt`, `log-after-blocked.txt` | `git log --oneline` after the commit, and after the blocked attempt |
| `status-before.txt`, `status-after.txt`, `status-after-blocked.txt` | `git status --short` before the commit, after it, and after the blocked attempt |

A `.frames.txt` file holds the screen at the end of each of hk's redraws. hk
wraps each redraw in synchronized output, and a frame is taken at each
`ESC[?2026l`. Frames are numbered from 0, and `kit/screens.ts` keeps those
numbers. A `.screen.txt` file is the final screen with its scrollback. Lines
are right-trimmed.

## How they were made

- **hk:** a debug build of hk 2.3.1 with the `check_diff` work that follows
  it: `main` at `388e8a72`, which has #1519 (a `check_diff` that only reads
  runs under read locks, and takes write locks only to apply its patch) and
  #1520 (hk applies `check_diff` patches itself instead of running
  `git apply`), with #1524 at `f5e221bf` (staging files a patch creates) and
  #1528 at `3428e391` (`trailing_whitespace` and `newlines` apply their
  diffs) merged in, and #1530's fix (`52ef7296`) on top. All four have
  since merged to `main` (`f461cc24`), which differs from this build only in
  a unit test, the paths of files a patch creates and the quoting of file
  names with tabs or quotes, none of which these runs touch. It reports
  version `2.3.1`. The reel shows that version (`VERSION` in
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
  the hashes are always `136d59a` and `6697300`. The blocked attempt ran at
  1790423899.
- **Records beside the terminal:** each run also wrote clx's frame log
  (`CLX_TRACE_LOG`, the job tree of every frame drawn), `script`'s timing
  file (`script -T`) and hk's per-step timing report (`HK_TIMING_JSON`).
  The runs that pass also wrote hk's log at trace level (`HK_LOG_FILE`,
  `HK_LOG_FILE_LEVEL=trace`). `blocked` ran without it, because a failing
  run ends its terminal output differently when the log file is at debug
  level or above. None of these writes to the terminal. Only the commit's
  lock log is committed (see below).

The demo repository's `hk.pkl` has seven steps, and `ruff-format` depends on
`ruff`. It amends a copy of the build's own `pkl/` directory at a fixed
path, because no released package has these builtins yet, and a fixed path
keeps `hk.pkl`, and with it the commit hashes, the same from run to run:

```pkl
amends "/dev/shm/hk-pkl/Config.pkl"
import "/dev/shm/hk-pkl/Builtins.pkl"

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

## The diffs

`ruff-format.diff.txt` is the standard output of the `ruff_format`
builtin's `check_diff` command, `ruff format --quiet --force-exclude --diff
src/main.py`, run by ruff 0.16.8 on a `src/main.py` holding exactly
`MAIN_PY_STAGED` (`kit/card.ts`), the staged file the stash scene shows. It
exited 1. As in the other captures, lines are right-trimmed, so the context
line for a blank line is empty, and the blank lines at the end are dropped:
the hunk's last two context lines are blank.

`ruff-format.after-ruff.diff.txt` is the same command's output once ruff
has fixed the file. `ruff-format` depends on `ruff`, and in a fix run hk
applies ruff's own diff first, which removes the unused `import os` (the
commit's and the fix run's logs both show `ruff: applied diff to 1 file(s)`
before `$ ruff format`). It was made the same way from `MAIN_PY_STAGED`:
`ruff check --force-exclude --diff --exit-non-zero-on-fix src/main.py`
applied with `git apply -p0`, then the `ruff format` command above, which
exited 1.

`test/diff.test.ts` checks the first diff against `MAIN_PY_STAGED`, the
second against `MAIN_PY_STAGED` without its import and against
`MAIN_PY_FIXED`, and that the everywhere scene's cards (`RUFF_FORMAT_DIFF`
under hk check, `RUFF_FORMAT_FIX_DIFF` under hk fix) show their lines
verbatim.

hk runs that command in both modes. In a scratch repository holding only
`MAIN_PY_STAGED` and a `ruff-format` step, the capture build's
`hk check --all` printed the first diff and exited 1. `hk fix --all` ran
the same command, applied the patch itself and exited 0, without running
the builtin's `fix` command: its log reads
`ruff-format: applied diff to 1 file(s)` and
`ruff-format: diff applied successfully, skipping fixer`. The builtin
declares the command `effect = "read"`, so hk holds only a read lock on the
file while it runs, and a write lock while it applies the patch.

## The lock log

`commit.locks.txt` is the commit run's trace log, cut to the lines that
order its steps' hold on their files: the commands hk ran for its steps (its
own `git` commands left out), its waits for locks and for `depends`, and
each `check_diff` outcome, with the timestamps and levels removed:

```sh
sed -E 's/^[0-9-]+ [0-9:]+ [A-Z]+ //' "$HK_LOG_FILE" \
  | grep -E '^\$ |waiting|check_diff succeeded|failed check step first|applied diff to' \
  | grep -v '^\$ git '
```

A step that `failed check step first` has a patch to apply, and
`applied diff to N file(s)` is hk applying it under write locks. A step
whose `check_diff succeeded` found nothing to fix and never took a write
lock. `lanes.test.ts` holds the lanes scene to this order: shfmt and
shellcheck both start reading `scripts/deploy.sh`, shfmt has its patch
before shellcheck has finished, and shfmt applies its patch only after.
The `failed to get … locks` lines name no step, so the test does not read
them. Two holds they reflect are left off the board: the read locks a step
takes to stage its files, and ruff letting `src/main.py` go after applying
its patch and taking it again to recheck the file.

## Timing

hk runs steps at once, so a repeat can show steps finishing in a different
order. These captures are the first of five runs of the same script. In all
five, the commit started prettier, ruff, shfmt and shellcheck together,
finished shellcheck, shfmt, ruff, ruff-format and prettier in that order,
and logged the same `check_diff` order for those steps. trailing-whitespace
and newlines, which run together, finished and logged their outcomes in
either order: trailing-whitespace first in three of the five. A row can change a frame earlier or later from run to
run. In the other captures, only steps that finish within a few
milliseconds of each other swapped places.

## Raw captures

The raw `script` output is not committed. Its carriage returns and cursor
movements fail the repository's trailing-whitespace check. `styles.txt` keeps
what the test needs from it: each distinct printed line, with the carriage
returns and the cursor, clear and synchronized-output sequences removed, and
the colour codes kept.
