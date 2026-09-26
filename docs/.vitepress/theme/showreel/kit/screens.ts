// K2's data: the terminal text the reel redraws, line for line as hk
// printed it in real runs at 80 columns (test/captures; its README says how
// they were made). Nothing here draws. term.ts derives every colour from the
// text itself, as the terminal showed it, and screens.test.ts checks every
// string below against the captures.
//
// Frames keep the capture's own numbers, so `commit[19]` is frame 19 of
// test/captures/commit.frames.txt: the screen at the end of one of hk's
// redraws. Only the frames the storyboard uses are here.

/**
 * The hk version the captures were made with, which every header shows.
 * Fixed rather than read from Cargo.toml: the terminal redraws those runs,
 * and a release should not re-render the video.
 */
export const VERSION = "2.2.0";

/** One screen: its lines from the top, as hk left them. */
export type Screen = readonly string[];

/**
 * Row 0 of the commit's pane: the command the user types. Typed, not
 * printed by hk, so it is the one terminal line that is not in a capture;
 * its message is the commit's (`[main ada2ca4] feat: hoist the sails`).
 */
export const PROMPT_COMMIT = '$ git commit -m "feat: hoist the sails"';

/**
 * `git commit -m "feat: hoist the sails"` through the pre-commit hook
 * (commit.frames.txt): 0 fetching git status, 1 the four staged files,
 * 2 the unstaged line stashed, 19 the stash restored, 22 the commit made.
 */
export const commit = {
  0: [
    "hk 2.2.0 by @jdx – pre-commit – fix  [                                     ] 0/7",
    "⠋ files - Fetching git status",
  ],
  1: [
    "hk 2.2.0 by @jdx – pre-commit – fix  [                                     ] 0/7",
    "✔ files - Fetching staged files (4 files)",
  ],
  2: [
    "hk 2.2.0 by @jdx – pre-commit – fix  [                                     ] 0/7",
    "✔ stash – Stashed unstaged changes (1 file)",
  ],
  19: [
    "hk 2.2.0 by @jdx – pre-commit – fix  [=====================================] 7/7",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ prettier  – 2 files modified – README.md, src/app.ts",
    "✔ ruff  – 1 file modified – src/main.py",
    "✔ ruff-format  – 1 file modified – src/main.py",
    "✔ shfmt  – 1 file modified – scripts/deploy.sh",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
    "✔ stash – Restoring unstaged changes (manual)",
  ],
  22: [
    "hk 2.2.0 by @jdx – pre-commit – fix  [=====================================] 7/7",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ prettier  – 2 files modified – README.md, src/app.ts",
    "✔ ruff  – 1 file modified – src/main.py",
    "✔ ruff-format  – 1 file modified – src/main.py",
    "✔ shfmt  – 1 file modified – scripts/deploy.sh",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
    "✔ stash – Restoring unstaged changes (manual)",
    "[main ada2ca4] feat: hoist the sails",
    " 3 files changed, 8 insertions(+), 1 deletion(-)",
  ],
} as const satisfies Readonly<Record<number, Screen>>;

/**
 * A later commit hk can't fix (blocked.frames.txt): shellcheck finds an
 * unused variable in the staged scripts/deploy.sh. 11 `✗ shellcheck`,
 * 12 `✗ shellcheck  – ERROR` at 4/4, 13 the stash restored. hk exits 1 and
 * git makes no commit.
 */
export const blocked = [
  // 0
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [                                     ] 0/7",
    "⠋ files - Fetching git status",
  ],
  // 1
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [                                     ] 0/7",
    "✔ files - Fetching staged files (1 file)",
  ],
  // 2
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [                                     ] 0/7",
    "✔ stash – Stashed unstaged changes (1 file)",
  ],
  // 3
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [                                     ] 0/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "❯ shellcheck",
  ],
  // 4
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [                                     ] 0/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "❯ shfmt",
    "❯ shellcheck",
  ],
  // 5
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [========>                            ] 1/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ shfmt",
    "❯ shellcheck",
    "❯ trailing-whitespace",
    " ⠋",
  ],
  // 6
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [========>                            ] 1/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ shfmt",
    "❯ shellcheck",
    "❯ trailing-whitespace",
  ],
  // 7
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [==================>                  ] 2/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ shfmt",
    "❯ shellcheck",
    "✔ trailing-whitespace",
    "❯ newlines",
    " ⠋",
  ],
  // 8
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [==================>                  ] 2/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ shfmt",
    "❯ shellcheck",
    "✔ trailing-whitespace",
    "❯ newlines",
  ],
  // 9
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [===========================>         ] 3/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ shfmt",
    "❯ shellcheck",
    " ⠋",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 10
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [===========================>         ] 3/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ shfmt",
    "❯ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 11
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [===========================>         ] 3/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ shfmt",
    "✗ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 12
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [=====================================] 4/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ shfmt",
    "✗ shellcheck  – ERROR",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 13
  [
    "hk 2.2.0 by @jdx – pre-commit – fix  [=====================================] 4/4",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ shfmt",
    "✗ shellcheck  – ERROR",
    "✔ trailing-whitespace",
    "✔ newlines",
    "✔ stash – Restoring unstaged changes (manual)",
  ],
] as const satisfies readonly Screen[];

/**
 * `hk check --all` on the clean repo after the commit (check-all.frames.txt):
 * 2 all seven checks reading at once, 15 all done at 7/7, 16 held.
 */
export const checkAll = [
  // 0
  [
    "hk 2.2.0 by @jdx – check  [                                                ] 0/7",
    "⠋ files - Fetching git status",
  ],
  // 1
  [
    "hk 2.2.0 by @jdx – check  [                                                ] 0/7",
    "✔ files - Fetching all files in repo (5 files)",
  ],
  // 2
  [
    "hk 2.2.0 by @jdx – check  [                                                ] 0/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "❯ ruff",
    " ⠋ ruff check --force-exclude src/main.py",
    "❯ shfmt",
    "❯ shellcheck",
    " ⠋ shellcheck scripts/deploy.sh",
    "❯ trailing-whitespace",
    " ⠋ hk util trailing-whitespace --diff README.md hk.pkl scripts/deploy.sh src/ap…",
    "❯ newlines",
    " ⠋ hk util end-of-file-fixer --diff README.md hk.pkl scripts/deploy.sh src/app.…",
  ],
  // 3
  [
    "hk 2.2.0 by @jdx – check  [======>                                         ] 1/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "❯ ruff",
    " ⠋ ruff check --force-exclude src/main.py",
    "✔ shfmt",
    "❯ shellcheck",
    " ⠋ shellcheck scripts/deploy.sh",
    "❯ trailing-whitespace",
    " ⠋ hk util trailing-whitespace --diff README.md hk.pkl scripts/deploy.sh src/ap…",
    "❯ newlines",
  ],
  // 4
  [
    "hk 2.2.0 by @jdx – check  [======>                                         ] 1/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "❯ ruff",
    " ⠋ ruff check --force-exclude src/main.py",
    "✔ shfmt",
    "❯ shellcheck",
    " ⠋ shellcheck scripts/deploy.sh",
    "❯ trailing-whitespace",
    " ⠋ hk util trailing-whitespace --diff README.md hk.pkl scripts/deploy.sh src/ap…",
    "❯ newlines",
  ],
  // 5
  [
    "hk 2.2.0 by @jdx – check  [=============>                                  ] 2/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "❯ ruff",
    " ⠋ ruff check --force-exclude src/main.py",
    "✔ shfmt",
    "❯ shellcheck",
    " ⠋ shellcheck scripts/deploy.sh",
    "❯ trailing-whitespace",
    "✔ newlines",
  ],
  // 6
  [
    "hk 2.2.0 by @jdx – check  [=============>                                  ] 2/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "❯ ruff",
    " ⠋ ruff check --force-exclude src/main.py",
    "✔ shfmt",
    "❯ shellcheck",
    " ⠋ shellcheck scripts/deploy.sh",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 7
  [
    "hk 2.2.0 by @jdx – check  [====================>                           ] 3/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "❯ ruff",
    " ⠋ ruff check --force-exclude src/main.py",
    "✔ shfmt",
    "❯ shellcheck",
    " ⠋ shellcheck scripts/deploy.sh",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 8
  [
    "hk 2.2.0 by @jdx – check  [====================>                           ] 3/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "❯ ruff",
    " ⠋ ruff check --force-exclude src/main.py",
    " All checks passed!",
    "✔ shfmt",
    "❯ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 9
  [
    "hk 2.2.0 by @jdx – check  [====================>                           ] 3/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "❯ ruff",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 10
  [
    "hk 2.2.0 by @jdx – check  [==========================>                     ] 4/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "✔ ruff",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 11
  [
    "hk 2.2.0 by @jdx – check  [=================================>              ] 5/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "✔ ruff",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 12
  [
    "hk 2.2.0 by @jdx – check  [=================================>              ] 5/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "✔ ruff",
    "❯ ruff-format",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 13
  [
    "hk 2.2.0 by @jdx – check  [========================================>       ] 6/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    " ⠋ prettier --check README.md src/app.ts",
    "✔ ruff",
    "✔ ruff-format",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 14
  [
    "hk 2.2.0 by @jdx – check  [========================================>       ] 6/7",
    "✔ files - Fetching all files in repo (5 files)",
    "❯ prettier",
    "✔ ruff",
    "✔ ruff-format",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 15
  [
    "hk 2.2.0 by @jdx – check  [================================================] 7/7",
    "✔ files - Fetching all files in repo (5 files)",
    "✔ prettier",
    "✔ ruff",
    "✔ ruff-format",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  // 16
  [
    "hk 2.2.0 by @jdx – check  [================================================] 7/7",
    "✔ files - Fetching all files in repo (5 files)",
    "✔ prettier",
    "✔ ruff",
    "✔ ruff-format",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
] as const satisfies readonly Screen[];

/**
 * The last screen of each run (the `.screen.txt` captures): the commit
 * (frame 22), `hk fix` in a working tree with the same problems unstaged,
 * and `hk check --all`.
 */
export const final = {
  commit: [
    "hk 2.2.0 by @jdx – pre-commit – fix  [=====================================] 7/7",
    "✔ stash – Stashed unstaged changes (1 file)",
    "✔ prettier  – 2 files modified – README.md, src/app.ts",
    "✔ ruff  – 1 file modified – src/main.py",
    "✔ ruff-format  – 1 file modified – src/main.py",
    "✔ shfmt  – 1 file modified – scripts/deploy.sh",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
    "✔ stash – Restoring unstaged changes (manual)",
    "[main ada2ca4] feat: hoist the sails",
    " 3 files changed, 8 insertions(+), 1 deletion(-)",
  ],
  fix: [
    "hk 2.2.0 by @jdx – fix  [==================================================] 7/7",
    "✔ files - Fetching modified files (4 files)",
    "✔ prettier",
    "✔ ruff",
    "✔ ruff-format",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
  checkAll: [
    "hk 2.2.0 by @jdx – check  [================================================] 7/7",
    "✔ files - Fetching all files in repo (5 files)",
    "✔ prettier",
    "✔ ruff",
    "✔ ruff-format",
    "✔ shfmt",
    "✔ shellcheck",
    "✔ trailing-whitespace",
    "✔ newlines",
  ],
} as const satisfies Readonly<Record<string, Screen>>;

/**
 * The finding in `blocked`, in shellcheck's own words (blocked.screen.txt):
 * the file, its offending line, and the start of the warning under it.
 */
export const finding = {
  file: "scripts/deploy.sh",
  source: "unused=1",
  warning: "^----^ SC2034 (warning)",
} as const;

/** `git log --oneline` after the commit (log.txt), newest first. */
export const LOG = ["ada2ca4 feat: hoist the sails", "f92f487 initial commit"] as const;
