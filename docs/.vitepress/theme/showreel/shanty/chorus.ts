// The chorus, four times over: the crew hauling on hk, a line at a time.
//
//   "Heave away, haul away, hk!"  the wordmark's hook swings to each heave
//                                 and haul, and glints on "hk"
//   "All hands haul at once!"     the lanes draw in, and on "haul" every
//                                 fixer starts and every padlock shuts at
//                                 once (lanes.ts's "all hands")
//   "For a lock on each file      the padlocks punch one a word, and on
//    takes the strain,"           "strain" the fixers finish, ✔ down the
//                                 lanes, and let their files go
//   "And we're bound away         the lanes give way to the main line, and
//    for the main!"               the commit sails along it onto its
//                                 place, landing on "main"
//
// The last time the line is "But she never made the main!": deploy.sh's
// commit sails up to the ✗ at its place and stops short. Every cue is a
// sung word's time (song.ts), so each chorus keeps its own singers' timing.

import { PALETTE, type SceneEnv } from "../bible";
import { glow, ring } from "../fx";
import { drawBar, drawDone, drawLanes, drawPadlock, LANES } from "../kit/lanes";
import { drawLogo, glint, LOGO_FULL, LOGO_OPEN, LOGO_POINT, logoToPx, sparkle, swingDeg } from "../kit/logo";
import { drawCommitDot, drawMain, MAIN } from "../kit/mainline";
import { BEAT } from "../timeline";
import { inOutCubic, lerp, outBack, progress, pulse, smoothstep, swiftOut } from "../math";
import { bump } from "../kit/motion";
import { drawCross } from "../scenes/catch-rig";
import { drawSea } from "./sea";
import type { Shot } from "./shot";
import { sungAt } from "./song";

type ChorusId = "chorus-1" | "chorus-2" | "chorus-3" | "final-chorus";

/** The fixers on the lanes: prettier across README.md and src/app.ts, ruff on main.py, shfmt on deploy.sh. */
const FIXERS = [
  { label: "prettier", lanes: [0, 1], to: 1480, curve: 0.9 },
  { label: "ruff", lanes: [2, 2], to: 1150, curve: 2.2 },
  { label: "shfmt", lanes: [3, 3], to: 1020, curve: 2.8 },
] as const;
/** Bars start 8 px inside the track. */
const BAR_X0 = LANES.trackX0 + 8;
/** The ✔ column, as the showreel's lanes cascade them. */
const DONE_X = 1730;

/** The chorus's cues: when each word that moves something is sung. */
function cues(id: ChorusId, last: boolean) {
  const l1 = `${id}.1`;
  const l2 = `${id}.2`;
  const l3 = `${id}.3`;
  const l4 = `${id}.4`;
  return {
    heave: sungAt(l1, "Heave"),
    haul1: sungAt(l1, "haul"),
    hk: sungAt(l1, "aitch-kay"),
    heave2: sungAt(l1, "Heave", 1),
    ho: sungAt(l1, "ho"),
    all: sungAt(l2, "All"),
    haul: sungAt(l2, "haul"),
    once: sungAt(l2, "once"),
    haulAway: sungAt(l2, "Haul"),
    locks: [sungAt(l3, "lock"), sungAt(l3, "on"), sungAt(l3, "each"), sungAt(l3, "file")],
    strain: sungAt(l3, "strain"),
    and: sungAt(l4, last ? "But" : "And"),
    bound: sungAt(l4, last ? "never" : "bound"),
    away: sungAt(l4, last ? "made" : "away"),
    main: sungAt(l4, "main"),
  };
}
type Cues = ReturnType<typeof cues>;

/** The hook's swing, degrees: a damped pendulum from each heave and haul, alternating sides. */
function swingAt(c: Cues, t: number): number {
  return swingDeg(t, c.heave, 11, 2 * BEAT) - swingDeg(t, c.haul1, 11, 2 * BEAT) + swingDeg(t, c.heave2, 8, 2 * BEAT);
}

/** "Heave away, haul away, hk!": the wordmark, the hook swinging to the words, glinting on "hk". */
function drawHeave(ctx: CanvasRenderingContext2D, c: Cues, t: number): void {
  // Up and away on the pickup to "All hands", clear before the lanes draw in.
  const leave = smoothstep(c.all - 0.35, c.all + 0.05, t);
  if (leave >= 1) return;
  // Each heave and haul lifts the mark, as a pull on a line would, and lets it back.
  const accents = [c.heave, c.haul1, c.hk, c.heave2, c.ho];
  const lift = accents.reduce((sum, at) => sum + bump(t, at, 1.5 * BEAT / 2), 0);
  const place = { ...LOGO_OPEN, cy: LOGO_OPEN.cy - 18 * lift - 120 * swiftOut(leave) };
  const swing = swingAt(c, t);
  const bloom = pulse(t, c.hk, 0.04, 0.5) + 0.6 * pulse(t, c.ho, 0.04, 0.35) + 0.25 * lift;
  ctx.save();
  ctx.globalAlpha *= 1 - leave;
  glow(ctx, place.cx, place.cy, 420, PALETTE.logo, 0.1 + 0.3 * bloom);
  drawLogo(ctx, place, LOGO_FULL, { swing });
  // A glint runs the leg on "hk" and again on "ho", into a sparkle on the point.
  for (const at of [c.hk, c.ho]) {
    const u = progress(at, at + 1.5 * BEAT, t);
    if (u > 0 && u < 1) glint(ctx, place, u, { swing });
  }
  const pt = logoToPx(place, LOGO_POINT[0], LOGO_POINT[1], swing);
  sparkle(ctx, pt.x, pt.y, progress(c.hk + BEAT, c.hk + 2 * BEAT, t));
  ctx.restore();
}

/** Lanes 0..3: tracks, labels, padlocks, and the fixers' bars. */
function drawHauling(ctx: CanvasRenderingContext2D, c: Cues, t: number): void {
  const enter = smoothstep(c.all - 0.05, c.haul - 0.05, t);
  // The lanes give way to the main line on the last line's first word.
  const leave = smoothstep(c.and - 0.1, c.bound, t);
  if (enter <= 0 || leave >= 1) return;
  ctx.save();
  ctx.globalAlpha *= 1 - leave;
  ctx.translate(0, 40 * inOutCubic(leave));
  drawLanes(ctx, {
    labels: enter,
    chars: LANES.rows.map((_, i) => 40 * progress(c.all + 0.06 * i, c.haul - 0.1, t)),
    tracks: LANES.rows.map((_, i) => swiftOut(progress(c.all - 0.05 + 0.05 * i, c.haul, t))),
    locks: 0,
  });
  // Every fixer starts on "haul" and finishes on "strain", prettier the long pole.
  const run = progress(c.haul, c.strain, t);
  const done = t >= c.strain;
  const flash = pulse(t, c.haulAway, 0.03, 0.3);
  for (const f of FIXERS) {
    const y0 = LANES.rows[f.lanes[0]] - LANES.barH / 2;
    const y1 = LANES.rows[f.lanes[1]] + LANES.barH / 2;
    const k = done ? 1 : 1 - (1 - run) ** f.curve;
    const x1 = lerp(BAR_X0, f.to, t < c.haul ? 0 : k);
    if (x1 > BAR_X0 + 1) {
      drawBar(ctx, { x0: BAR_X0, x1, y0, y1 }, "fix", { label: f.label, fullWidth: f.to - BAR_X0 });
      if (flash > 0.01) glow(ctx, x1, (y0 + y1) / 2, 90, PALETTE.warm, 0.5 * flash);
    }
  }
  // Every padlock shuts on "haul", punches on its word of "a lock on each
  // file", and opens again once its file is done.
  LANES.rows.forEach((cy, i) => {
    const shut = t >= c.haul && t < c.strain + (i + 1) * (BEAT / 4);
    const punch = pulse(t, c.haul, 0.02, 0.15) + pulse(t, c.locks[i], 0.02, 0.2);
    drawPadlock(ctx, LANES.lockX, cy, shut ? "write" : "open", {
      alpha: enter,
      lift: shut ? 1 - swiftOut(progress(c.haul, c.haul + BEAT / 4, t)) : 1,
      scale: 1 + 0.35 * punch,
    });
    if (punch > 0.02) glow(ctx, LANES.lockX, cy, 70, PALETTE.warm, 0.6 * punch);
    // The ✔ cascade, a lane per sixteenth from "strain".
    const at = c.strain + i * (BEAT / 4);
    if (t >= at) drawDone(ctx, DONE_X, cy, { size: 44, scale: outBack(2.2)(progress(at, at + BEAT / 4, t)) });
  });
  ctx.restore();
}

/** "Bound away for the main": the main line, and the commit sailing onto its place. */
function drawBound(ctx: CanvasRenderingContext2D, c: Cues, t: number, last: boolean): void {
  const on = smoothstep(c.and - 0.05, c.bound, t);
  if (on <= 0) return;
  const line = swiftOut(progress(c.and, c.bound + 0.3, t));
  // It sails from the left margin, and lands on "main".
  const sail = inOutCubic(progress(c.bound, c.main, t));
  const landed = t >= c.main;
  const x = lerp(MAIN.x0, last ? MAIN.head.x - 150 : MAIN.head.x, sail);
  drawMain(ctx, { line, parent: smoothstep(c.and, c.bound, t), head: 0, labels: last ? 0 : smoothstep(c.main, c.main + 0.3, t), alpha: on });
  if (last) {
    // The ✗ stands at its place from "never"; she stops short of it.
    const cross = outBack(2)(progress(c.bound, c.bound + BEAT / 2, t));
    if (cross > 0) {
      ctx.save();
      ctx.translate(MAIN.head.x, MAIN.y);
      ctx.scale(cross, cross);
      drawCross(ctx, 0, 0, 44, PALETTE.red, 0.14);
      ctx.restore();
      glow(ctx, MAIN.head.x, MAIN.y, 90, PALETTE.red, 0.35 * cross + 0.4 * pulse(t, c.main, 0.02, 0.3));
    }
    drawCommitDot(ctx, x, "slot", { alpha: on });
    ctx.save();
    ctx.globalAlpha *= on * (1 - 0.5 * smoothstep(c.main, c.main + 0.4, t));
    ctx.fillStyle = PALETTE.red;
    ctx.beginPath();
    ctx.arc(x, MAIN.y, MAIN.dotR * 0.8, 0, Math.PI * 2);
    ctx.fill();
    ctx.restore();
    return;
  }
  drawCommitDot(ctx, x, "head", { alpha: on, glow: landed ? 1 + 1.5 * pulse(t, c.main, 0.02, 0.4) : 0.6 });
  ring(ctx, MAIN.head.x, MAIN.y, 120, progress(c.main, c.main + 0.6, t), PALETTE.cyanBright, 5);
}

/** The chorus `id`, from `from` to `to` on the song's clock. */
export function chorus(id: ChorusId, span: { from: number; to: number }): Shot {
  const last = id === "final-chorus";
  const c = cues(id, last);
  return {
    name: id,
    start: span.from,
    end: span.to,
    draw(ctx, t, env: SceneEnv) {
      ctx.fillStyle = PALETTE.bg;
      ctx.fillRect(0, 0, env.W, env.H);
      drawSea(ctx, t, { alpha: 0.8 });
      drawHeave(ctx, c, t);
      drawHauling(ctx, c, t);
      drawBound(ctx, c, t, last);
    },
  };
}
