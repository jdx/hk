// What the landing page says each chapter of the reel shows, for readers who
// cannot watch it: HomeShowreel.vue lists these under the player. The only
// figures are the benchmark's, taken from the same facts the race scene
// draws and withheld with them, so the page never states a number the video
// does not show (test/describe.test.ts holds it to that). This module is
// server-rendered with the page, so it imports only facts.ts and timeline.ts.

import { type Race, races, type ReelFacts } from "./facts";
import { SECTIONS, type SectionId } from "./timeline";

/** "a, b and c". */
function list(items: readonly string[]): string {
  return items.length < 2 ? items.join("") : `${items.slice(0, -1).join(", ")} and ${items[items.length - 1]}`;
}

/**
 * One race as a sentence: every median the chart draws, in the page's
 * format, and the caption's ratio against the rival it names.
 */
function raceSentence(r: Race): string {
  const [hk, ...rivals] = r.rows;
  const times = list([`hk takes ${hk.shown}`, ...rivals.map((x) => `${x.label} ${x.shown}`)]);
  const task = r.title.charAt(0).toLowerCase() + r.title.slice(1);
  return `To ${task}, ${times}: ${r.claim.ratio} times faster than the fastest other tool, ${r.claim.rival.label} (${r.claim.rival.mode}).`;
}

/** The benchmark chapter, as the race scene draws it under `f`. */
function race(f: ReelFacts | null): string {
  const drawn = races(f);
  if (!f || !drawn.length) {
    return "A terminal runs hk check --all: its steps start together and all of them pass. Measured results are on hk.jdx.dev/benchmarks.";
  }
  const { files, fixers, cpus } = f.workload;
  const intro = `A bar chart race from hk's published benchmark: ${files} files, ${fixers} fixers, ${cpus} CPUs.`;
  const one = drawn.length === 1 ? " Runs are timed only when every file comes out right." : "";
  return `${intro} ${drawn.map(raceSentence).join(" ")}${one} Commit timings and every tool are on hk.jdx.dev/benchmarks.`;
}

/** What each chapter shows, in the reel's order. */
export function describeChapters(f: ReelFacts | null): { id: SectionId; label: string; text: string }[] {
  const text: Record<SectionId, string> = {
    open: "On a dark stage, a fishing hook drops in on a line while a pen draws the hk logo. The hook becomes the leg of the k and clicks into place. The caption reads: Git hooks and project checks, in parallel.",
    config:
      "Names of hk's builtin linter and formatter configurations stream across the screen, from prettier to zizmor. Seven of them fly into an hk.pkl file as steps: prettier, ruff, ruff-format, which depends on ruff, shfmt, shellcheck, trailing-whitespace and newlines. The caption reads: Configured in Pkl, typed and reusable.",
    commit:
      "The command git commit, with the message feat: hoist the sails, slams onto the screen. Pressing Enter opens a terminal where hk's pre-commit hook starts and fetches the staged files. The caption reads: After hk install, git commit runs hk before the commit.",
    stash:
      "The staged file src/main.py appears with one unstaged line at its end, a to-do comment. hk slices that line off into a git stash, so the linters see only the staged version, with its formatting problems underlined.",
    lanes:
      "The seven steps run as bars on four file lanes. prettier, ruff and shfmt start together on different files. shellcheck waits for shfmt on the same shell script, ruff-format waits for ruff because it depends on it, and trailing-whitespace and newlines, which touch every file, wait for the others and then take turns. A note says the order comes from one real commit and is not to scale.",
    restore:
      "The fixed files are staged, and README.md is back to its last committed version. Side by side, the staged src/main.py holds the fixed code, and the worktree copy gets the same fixes plus the to-do line back from the stash. Then the commit feat: hoist the sails lands on the main branch.",
    catch:
      "A later commit stages scripts/deploy.sh with an unused variable. hk's hook reports a shellcheck error, unused appears unused, which it cannot fix. The fishing hook snags the commit and lifts it off the branch, so no commit is made.",
    everywhere:
      "Three panels show the same seven steps passing in a git commit, in hk fix at a terminal, and in hk check --all in CI. Then two small timelines compare one file: under hk check, three checks read README.md at the same time; under hk fix, three fixers take turns.",
    race: race(f),
    morph: "The bars shrink into strokes that set into the hk logo.",
    end: "The hk logo returns in full, beside the name hk, the line Git hooks for linters and formatters, the command mise use hk, and the address hk.jdx.dev.",
  };
  return SECTIONS.map(({ id, label }) => ({ id, label, text: text[id] }));
}
