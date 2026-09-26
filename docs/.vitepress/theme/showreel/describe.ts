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
 * A caption as the page quotes it: its lines, joined, without the code
 * marks. Every caption the video shows is quoted where it appears, except a
 * race's claim, whose figure the race's sentence states in words.
 */
const caption = (...lines: string[]): string => `Caption: ${lines.join(" ")}`;

/**
 * One race as a sentence: every row the chart draws, with its mode and its
 * median in the page's format, and the claim's ratio against the fastest
 * other tool.
 */
function raceSentence(r: Race): string {
  const times = list(r.rows.map((x, i) => `${x.label} (${x.mode}) ${i === 0 ? "takes " : ""}${x.shown}`));
  const task = r.title.charAt(0).toLowerCase() + r.title.slice(1);
  return `To ${task}, ${times}: ${r.claim.ratio} times faster than the fastest other tool, ${r.claim.rival.label}.`;
}

/**
 * The benchmark chapter, as the race scene draws it under `f`. Without a
 * race to draw, whether the run is missing or backs no claim, the scene
 * replays hk's own check run and points to the page.
 */
function race(f: ReelFacts | null): string {
  const drawn = races(f);
  if (!f || !drawn.length) {
    return (
      "A terminal runs hk check --all: six of its seven steps start together, ruff-format follows ruff, and all seven pass. " +
      `${caption("Independent steps run in parallel.")} A note under the terminal reads Benchmarks: hk.jdx.dev/benchmarks.`
    );
  }
  const { files, fixers, cpus } = f.workload;
  const intro = `A bar chart race from hk's published benchmark: ${files} files, ${fixers} fixers, ${cpus} CPUs.`;
  const one = drawn.length === 1 ? ` ${caption("Timed only when the files are right.")}` : "";
  return `${intro} ${drawn.map(raceSentence).join(" ")}${one} Commit timings and every tool are on hk.jdx.dev/benchmarks.`;
}

/** What each chapter shows, in the reel's order. */
export function describeChapters(f: ReelFacts | null): { id: SectionId; label: string; text: string }[] {
  const text: Record<SectionId, string> = {
    open:
      "On a dark stage, a fishing hook drops in on a line while a pen draws the hk logo. The hook becomes the leg of the k and clicks into place. " +
      caption("Git hooks and project checks,", "in parallel."),
    config:
      "Names of hk's builtin linter and formatter configurations stream across the screen. " +
      `${caption("Builtins, from prettier to zizmor.")} ` +
      "Seven of them fly into an hk.pkl file as steps: prettier, ruff, ruff-format, which depends on ruff, shfmt, shellcheck, trailing-whitespace and newlines. " +
      caption("Configured in Pkl:", "typed and reusable."),
    commit:
      "The command git commit, with the message feat: hoist the sails, slams onto the screen. Pressing Enter opens a terminal where hk's pre-commit hook starts and fetches the staged files. " +
      caption("After hk install, git commit", "runs hk before the commit."),
    stash:
      "The staged file src/main.py appears with one unstaged line at its end, a to-do comment. hk slices that line off into a git stash, and the formatting problems in the staged version are underlined. " +
      caption("Unstaged work is stashed,", "so linters see only what you staged."),
    lanes:
      "The seven steps run as bars on four file lanes. prettier, ruff and shfmt start together on different files. " +
      `${caption("Different files? Steps run at once.")} ` +
      "shellcheck waits for shfmt on the same shell script, ruff-format waits for ruff because it depends on it, and trailing-whitespace and newlines, which touch every file, wait for the others and then take turns. " +
      `${caption("Same file? They take turns.")} A note says the order comes from one real commit and is not to scale.`,
    restore:
      "The fixed files are staged, and README.md is back to its last committed version. Side by side, the staged src/main.py holds the fixed code, and the worktree copy gets the same fixes plus the to-do line back from the stash. " +
      `${caption("Fixes are staged.", "Your edits come back.")} ` +
      `Then the commit feat: hoist the sails lands on the main branch. ${caption("The commit gets the fixed version.")}`,
    catch:
      "A later commit stages scripts/deploy.sh with an unused variable. shellcheck flags it with a warning, which hk cannot fix, so the step fails. " +
      `The fishing hook snags the commit before it reaches the branch, so no commit is made. ${caption("Can't be fixed?", "hk blocks the commit.")}`,
    everywhere:
      "Three panels show the same seven steps passing in a git commit, in hk fix at a terminal, and in hk check --all in CI. " +
      `${caption("One set of steps:", "commit, terminal, CI.")} ` +
      "Then two small timelines compare one file, README.md: under hk check, three checks read it at the same time; under hk fix, three fixers take turns. " +
      caption("Checks share a file.", "Fixes take turns."),
    race: race(f),
    morph: "The bars shrink into strokes that set into the hk logo.",
    end: "The hk logo returns in full, beside the name hk, the line Git hooks for linters and formatters, the command mise use hk, and the address hk.jdx.dev.",
  };
  return SECTIONS.map(({ id, label }) => ({ id, label, text: text[id] }));
}
