// Scene 9, "Benchmarks" (storyboard §6.9): the published results, stated
// exactly as the benchmarks page states them. The chart arrives on the
// whip from `everywhere`, its layers trailing in from the right, and races:
// hk and every other tool on the same clock, each bar stopping on its own
// median (race-chart.ts, on race-timing.ts's beats, which the score's dings
// share). Two scenarios are drawn, the commit and then Fix every file; the
// rest, Check every file included, is on the page, and the bottom detail
// points there. Then the labels wipe and the bars ease into race|morph's
// four capsules.
//
// Variants, by what the facts back (facts.ts):
// - both races: Commit from b1, a reset on b8.5 as the rows re-sort, Fix
//   every file from b9;
// - one (F1): that race from b1, then the finished chart holds, the bottom
//   detail rising on b8;
// - none (F0: no facts, or no claim): no figure at all. The whip brings in
//   hk's own terminal running `hk check --all` (checkAll frames 0–16, every
//   row of each), pointing at the benchmarks page under it, and the
//   capsules grow in from the axis as it fades, hk's last. Its only digits
//   are hk's output.
// Every variant starts on the whip's streaks alone and ends on the capsules.

import { BEAT, PALETTE, type ReelFacts, type Scene, type SceneEnv, sec } from "../bible";
import { claimLine, type Race, races } from "../facts";
import { raceRuns } from "../race-timing";
import { ring } from "../fx";
import { drawHandoff } from "../handoff";
import { checkAll } from "../kit/screens";
import { drawTerm, PANE_FULL, type Pane, termLayout, termLit } from "../kit/term";
import { rgba } from "../color";
import { clamp, inOutSine, outQuart, progress, smoothstep, swiftOut } from "../math";
import { type Caption, drawText, font, readingTime, wordCount } from "../type";
import { drawWhip, drawWhipIn, WHIP_END, whipIn } from "../whip";
import { chartModel, drawChart, growCapsule, REST } from "./race-chart";

const S = sec("race");

/** Local seconds of section beat `n`. */
const b = (n: number): number => n * BEAT;

/** A claim's second line. */
const CLAIM_TAIL = "than the fastest other tool.";
/** The first claim wipes on the reset, b8.5, with the figures it states. */
const FIRST_OUT = 8.5;
/**
 * The first claim lands on b2.5, or later so that its ratio (the line's
 * second-to-last word, rising a quarter beat before the line lands) starts
 * to rise only once the fastest other tool's bar has stopped and its median
 * is up: half a beat after that stop, on the half-beat grid. It must still
 * hold all its words by FIRST_OUT, so a longer claim lands no later than
 * that allows: "Fix every file: …" and its tail are 10 words, which need
 * 6 beats, so it lands on b2.5 and its ratio can lead the rival's median by
 * a fraction of a beat. "Commit: …" is 8, so it can wait for its rival.
 */
function firstClaimAt(r: Race, f: ReelFacts | null): number {
  const stop = raceRuns(f)[0].stops[r.claim.rival.key];
  const latest = Math.floor(2 * (FIRST_OUT - readingTime(wordCount(claimLine(r)) + wordCount(CLAIM_TAIL)) / BEAT)) / 2;
  return Math.max(2.5, Math.min(Math.ceil(2 * (stop + 0.5)) / 2, latest));
}

/** The section's must-read captions under `f`. */
export function captions(f: ReelFacts | null): Caption[] {
  const both = races(f);
  const claim = (r: Race, at: number, out: number): Caption => ({
    out,
    lines: [
      { in: at, text: claimLine(r) },
      { in: at + 0.5, text: CLAIM_TAIL },
    ],
  });
  // The first claim holds from its landing to the reset (firstClaimAt); the
  // second, 10 words, needs 6 beats and holds 6.
  if (both.length === 2) return [claim(both[0], firstClaimAt(both[0], f), FIRST_OUT), claim(both[1], 9.5, 15.5)];
  // 7 words: need 4.5, hold 5.75.
  if (both.length === 1) return [claim(both[0], firstClaimAt(both[0], f), FIRST_OUT), { out: 15.5, lines: [{ in: 9.75, text: "Timed only when the files are right." }] }];
  // 5 words: need 3.5, hold 9, over the run and its 7/7. No digits.
  return [{ out: 12, lines: [{ in: 3, text: "Independent steps run in parallel." }] }];
}

// F0: hk's terminal (storyboard §6.9 Variants).

/**
 * PANE_FULL's window and 32 px text, set a little closer (38 px baselines)
 * to hold 13 rows, so every screen of checkAll shows whole: frame 2's six
 * running steps included. It stands higher, y 102–664, so the pointer to
 * the page fits under it, above the captions' band.
 */
export const F0_PANE: Pane = { ...PANE_FULL, anchor: "top", y: 102, h: 562, lineH: 38, baseline0: 188, rows: 13 };
/** checkAll frame 0 lands on b1, and each next one 0.375 beats later: frame 16 on b7. */
export const F0_FIRST = b(1);
export const F0_EACH = b(0.375);
/** The pointer to the page: right-aligned under the pane, above the captions' band, rising on b8. */
export const F0_DETAIL = "Benchmarks: hk.jdx.dev/benchmarks";
export const F0_DETAIL_AT = { x: 1760, y: 712 } as const;
export const F0_DETAIL_IN = b(8);
/** The pane fades, settling back a little, as the capsules grow in. */
export const F0_OUT = [b(14.5), b(15)] as const;
/**
 * Each capsule's growth in F0, from the bottom up, overlapping the fade so
 * the stage is never empty: the other tools' three from b14.625, over
 * empty pane at 7/7, and hk's, just under the pane's top lines, once the
 * pane has all but gone. All are home by REST.
 */
export const F0_GROW = [b(14.9375), b(14.75), b(14.6875), b(14.625)] as const;
export const F0_GROW_DUR = b(0.75);
/** The run completes (frame 15, 7/7): a light runs along the header's bar. */
const F0_DONE = F0_FIRST + 15 * F0_EACH;

/** The checkAll frame on screen at `lt`. */
export const f0Frame = (lt: number): number => clamp(Math.floor((lt - F0_FIRST) / F0_EACH + 1e-9), 0, checkAll.length - 1);

/** The steps a screen shows as passed. */
const passed = (screen: readonly string[]): Set<string> => new Set(screen.filter((l) => l.startsWith("✔ ")).map((l) => l.split(" ")[1]));

/** How far the pane is in at `lt`: 0 on the bar line, 1 once the whip has cleared, then out. */
const f0Alpha = (lt: number): number => outQuart(progress(0, WHIP_END - S.start, lt)) * (1 - smoothstep(F0_OUT[0], F0_OUT[1], lt));

function drawF0(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  const f = f0Frame(lt);
  const lines = checkAll[f];
  const out = smoothstep(F0_OUT[0], F0_OUT[1], lt);
  const pane = () => {
    ctx.save();
    // Settles back a little as it goes, about its centre.
    const s = 1 - 0.02 * out;
    const cx = F0_PANE.x + F0_PANE.w / 2;
    const cy = F0_PANE.y + F0_PANE.h / 2;
    ctx.translate(cx, cy);
    ctx.scale(s, s);
    ctx.translate(-cx, -cy);
    const L = drawTerm(ctx, F0_PANE, lines, { t: env.t, alpha: 1 - out });
    // 7/7: a light runs along the header's full bar, once.
    const sweep = progress(F0_DONE, F0_DONE + 0.45, lt);
    if (sweep > 0 && sweep < 1) {
      const open = lines[0].indexOf("[");
      const close = lines[0].indexOf("]");
      const x0 = L.col(open);
      const x1 = L.col(close + 1);
      const cell = L.cell(0, 0);
      const cx = x0 + (x1 - x0 + 240) * inOutSine(sweep) - 120;
      ctx.save();
      ctx.beginPath();
      ctx.rect(x0, cell.y, x1 - x0, cell.h);
      ctx.clip();
      ctx.globalCompositeOperation = "lighter";
      const g = ctx.createLinearGradient(cx - 120, 0, cx + 120, 0);
      g.addColorStop(0, rgba(PALETTE.cyan, 0));
      g.addColorStop(0.5, rgba(PALETTE.cyan, 0.28));
      g.addColorStop(1, rgba(PALETTE.cyan, 0));
      ctx.fillStyle = g;
      ctx.fillRect(cx - 120, cell.y, 240, cell.h);
      ctx.restore();
    }
    // Each step that has just passed rings once, green, as its ✔ lands.
    if (out <= 0) {
      for (let k = Math.max(1, f - 2); k <= f; k++) {
        const at = F0_FIRST + k * F0_EACH;
        const u = progress(at, at + 0.32, lt);
        if (u <= 0 || u >= 1) continue;
        const before = passed(checkAll[k - 1]);
        const L = termLayout(F0_PANE, checkAll[k].length);
        checkAll[k].forEach((line, i) => {
          if (!line.startsWith("✔ ") || before.has(line.split(" ")[1]) || i >= F0_PANE.rows) return;
          // Its row in the frame on screen now, which may have moved up since.
          const row = lines.findIndex((l) => l === line);
          const c = L.cell(row < 0 ? i : row, 0);
          ring(ctx, c.x + c.w / 2, c.y + c.h / 2 - 2, 30, u, PALETTE.green, 3);
        });
      }
    }
    ctx.restore();
  };
  if (env.t < WHIP_END) drawWhipIn(ctx, env.t, pane);
  else pane();
  // The pointer to the page.
  const p = progress(F0_DETAIL_IN, F0_DETAIL_IN + 0.25, lt) * (1 - out);
  if (p > 0) {
    ctx.save();
    ctx.globalAlpha *= clamp(p / 0.6);
    drawText(ctx, F0_DETAIL, F0_DETAIL_AT.x, F0_DETAIL_AT.y + 12 * (1 - swiftOut(clamp(p))), { font: font(40, 500), fill: PALETTE.text3, align: "right" });
    ctx.restore();
  }
  for (let i = 0; i < 4; i++) growCapsule(ctx, i, lt, F0_GROW[i], F0_GROW_DUR);
}

// The scene.

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw(ctx, lt, env) {
    if (lt >= REST) {
      drawHandoff(ctx, "race|morph", env);
      return;
    }
    ctx.fillStyle = PALETTE.bg;
    ctx.fillRect(0, 0, env.W, env.H);
    const m = chartModel(env.facts);
    if (m) drawChart(ctx, m, lt);
    else drawF0(ctx, lt, env);
    // The whip's streaks, over whatever is arriving (nothing is drawn after b1).
    drawWhip(ctx, env.t);
  },
  lit(lt, env) {
    // F0's pane is lit while it is up, following the whip in and its fade.
    if (chartModel(env.facts) || lt >= REST) return null;
    const a = f0Alpha(lt);
    if (a <= 0) return null;
    return termLit({ ...F0_PANE, x: F0_PANE.x + whipIn(S.start + lt) }, a);
  },
  captions,
};
