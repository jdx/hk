// Where the morph's bars are (storyboard §6.10), as pure functions of the
// section's local time: the geometry of each bar's flight from its capsule
// in race|morph to its stroke of the wordmark at LOGO_END, and the two pens
// that write the rest. scenes/morph.ts draws them; test/morph.test.ts holds
// the flights to their shape (no bar through another, no "L").

import { BEAT } from "../bible";
import { capsuleAt } from "../handoff";
import { KLEG_RUN, LOGO_END, LOGO_STROKE, LOGO_STROKES, logoScale, logoToPx, STROKE } from "../kit/logo";
import { bump, type Pt } from "../kit/motion";
import { clamp, cubicBezier, DEG, inOutCubic, inOutSine, keys, lerp, outCubic, progress, smoothstep } from "../math";
import { DUB, LAND, LIFT, LUB, PEN, SWAP } from "./morph-cues";

/** Local seconds of section beat `n`. */
const b = (n: number): number => n * BEAT;

/** How long a landed bar rocks before it is still, seconds. */
export const ROCK = b(0.3);
/** When bar i is still on its stroke, and the logo kit draws it from then on. */
export const SETTLE = [LAND[0] + ROCK, LAND[1] + ROCK, LAND[2] + ROCK, SWAP] as const;

// Geometry.

export interface Pose {
  /** Cap centres: the tail and the head. */
  a: Pt;
  b: Pt;
  width: number;
}

/** The wordmark's stroke at LOGO_END, px (39.3). */
export const LOGO_W = LOGO_STROKE * logoScale(LOGO_END);

/** A straight run of the wordmark at LOGO_END, tail to head, in logo units. */
const seg = (x0: number, y0: number, x1: number, y1: number): Pose => ({
  a: logoToPx(LOGO_END, x0, y0),
  b: logoToPx(LOGO_END, x1, y1),
  width: LOGO_W,
});

/**
 * Where each bar lands (storyboard §6.10). The head is the end that leads,
 * the bar's right end: the stems stand up head first on their feet, the arm
 * points up out of the stem, and the leg's run is drawn out from its root.
 */
export const TARGET: readonly Pose[] = [seg(28, 80, 28, 16), seg(92, 80, 92, 16), seg(92, 68, 126, 46), seg(100, 62, 122, 76)];
/** The logo stroke each bar becomes. */
export const BECOMES = [STROKE.hStem, STROKE.kStem, STROKE.kArm, STROKE.kLeg] as const;
/** The leg's progress at the end of its straight run (0.2813). */
export const RUN_P = KLEG_RUN / LOGO_STROKES[STROKE.kLeg].len;

/** The four capsules of race|morph. */
export const START: readonly Pose[] = [0, 1, 2, 3].map((i) => {
  const c = capsuleAt(i);
  return { a: c.a, b: c.b, width: c.width };
});

export const angleOf = (p: Pose): number => Math.atan2(p.b.y - p.a.y, p.b.x - p.a.x);
export const lengthOf = (p: Pose): number => Math.hypot(p.b.x - p.a.x, p.b.y - p.a.y);
export const mid = (p: Pose): Pt => ({ x: (p.a.x + p.b.x) / 2, y: (p.a.y + p.b.y) / 2 });

/** `p` turned by `th` radians about `o`. */
function turn(p: Pt, o: Pt, th: number): Pt {
  const c = Math.cos(th);
  const s = Math.sin(th);
  return { x: o.x + (p.x - o.x) * c - (p.y - o.y) * s, y: o.y + (p.x - o.x) * s + (p.y - o.y) * c };
}

// Motion. A bar's flight runs from LIFT to LAND as progress p, 0 to 1.

/**
 * The gather: the bar shortens toward the column of its stroke, sliding
 * right along its own row, fast out and done by 55% of the flight.
 */
const slide = cubicBezier(0.35, 0, 0.1, 1);
const gathered = (p: number): number => slide(clamp(p / 0.55));

/**
 * The rise from its row to its stroke, in and out: the stems and the pen
 * over the whole flight, home on their sixteenths. The arm rises sooner,
 * once its tail is past the stem's foot, so it is up by the stem as a k's
 * arm while its root docks, rather than lying at the foot.
 */
const riseLate = inOutCubic;
const RISE_SOON = [0.04, 0.9] as const;
const riseSoon = (p: number): number => inOutCubic(clamp((p - RISE_SOON[0]) / (RISE_SOON[1] - RISE_SOON[0])));

/**
 * The turn to its stroke's angle, with a dip the other way while it
 * gathers (a share ANTICIPATE of the turn). The stems and the pen are square
 * by 90% of the flight. The arm turns sooner, square by 58%: it is already
 * its diagonal when its tail comes up by the k's stem, so the two never
 * read as an "L" on the way in.
 */
const ANTICIPATE = 0.04;
const turnLate = keys([
  [0, 0],
  [0.22, -ANTICIPATE, inOutSine],
  [0.9, 1, inOutCubic],
]);
const turnSoon = keys([
  [0, 0],
  [0.12, -ANTICIPATE, inOutSine],
  [0.58, 1, inOutCubic],
]);

/** How far right of the arm's root its tail gathers, px: its ink stays clear of the stem's far side. */
const DOCK = 36;
/** The dock: back from there onto the root, from halfway through the flight, home on the landing. */
const docking = (p: number): number => smoothstep(0.5, 1, p);

/**
 * Each bar's character: how it rises and turns; how far past its angle it
 * swings (a share of its turn: 1.8° for the stems, 3.3° for the arm),
 * reached on its landing and rocked out over ROCK; how far it leans into
 * the climb, degrees; and, for a bar placed by its tail, how far right of
 * its root the tail gathers to, px, so it docks from the right (the arm,
 * onto the k's stem, never through it).
 */
const FLIGHT = [
  { rise: riseLate, turn: turnLate, over: 0.02, lean: 0, dock: 0 },
  { rise: riseLate, turn: turnLate, over: 0.02, lean: 0, dock: 0 },
  { rise: riseSoon, turn: turnSoon, over: 0.1, lean: 0, dock: DOCK },
  { rise: riseLate, turn: turnLate, over: 0, lean: -14, dock: 0 },
] as const;

/**
 * The swing past the stroke's angle, as a share of it: it builds over the
 * last third of the flight to its peak on the landing, then rocks back as a
 * damped spring from rest there, exactly 0 from ROCK after it.
 */
function overswing(lt: number, i: number): number {
  const over = FLIGHT[i].over;
  if (!over) return 0;
  if (lt < LAND[i]) return over * smoothstep(0.65, 1, progress(LIFT[i], LAND[i], lt));
  const d = lt - LAND[i];
  if (d >= ROCK) return 0;
  const k = 10;
  const w = (2 * Math.PI) / 0.17;
  return over * Math.exp(-k * d) * (Math.cos(w * d) + (k / w) * Math.sin(w * d)) * (1 - smoothstep(0.55 * ROCK, ROCK, d));
}

/** The pen's shortest, as a share of the run, and when it starts to draw out, as flight progress. */
const PEN_LEAST = 0.62;
const PEN_DRAW = 0.8;

/** The pen's length: it gathers short, holds, and draws out along the run as its tail docks, faster and faster. */
function penLength(p: number): number {
  const run = lengthOf(TARGET[3]);
  const least = PEN_LEAST * run;
  if (p < PEN_DRAW) return lerp(lengthOf(START[3]), least, gathered(p));
  const q = (p - PEN_DRAW) / (1 - PEN_DRAW);
  return lerp(least, run, q * q);
}

/** The heartbeat, felt in the bars still waiting: a swell on the lub, a smaller one on the dub. */
const throb = (lt: number): number => 1 + 0.07 * bump(lt, LUB[0], b(0.3)) + 0.04 * bump(lt, DUB[0], b(0.3));

/** Bar i at `lt`, from its lift until it is still on its stroke. */
export function poseAt(i: number, lt: number): Pose {
  const s = START[i];
  const e = TARGET[i];
  if (lt <= LIFT[i]) return { ...s, width: s.width * throb(lt) };
  if (lt >= LAND[i]) {
    // Landed, and rocking on its tail (a foot, or the arm's root) until still.
    const th = (angleOf(e) - angleOf(s)) * overswing(lt, i);
    return { a: e.a, b: turn(e.b, e.a, th), width: e.width };
  }
  const p = progress(LIFT[i], LAND[i], lt);
  const f = FLIGHT[i];
  const g = gathered(p);
  const up = f.rise(p);
  const turned = angleOf(s) + (angleOf(e) - angleOf(s)) * f.turn(p) + f.lean * DEG * bump(p, 0.25, 0.75);
  const dir = { x: Math.cos(turned), y: Math.sin(turned) };
  const width = lerp(s.width, e.width, smoothstep(0.1, 0.9, p)) * throb(lt);
  // The swing past its angle turns it on its tail, as the rock after landing does.
  const th = (angleOf(e) - angleOf(s)) * overswing(lt, i);
  if (i >= 2) {
    // The arm and the pen are placed by their tails, which lead: each
    // gathers toward its tail, which slides along its row and rises onto its
    // root, the arm's from the right of the stem. The pen's head then draws
    // out along the leg's run.
    const len = i === 3 ? penLength(p) : lerp(lengthOf(s), lengthOf(e), g);
    const tail = { x: lerp(s.a.x, e.a.x + f.dock, g) - f.dock * docking(p), y: lerp(s.a.y, e.a.y, up) };
    const head = { x: tail.x + dir.x * len, y: tail.y + dir.y * len };
    return { a: tail, b: th ? turn(head, tail, th) : head, width };
  }
  // The stems gather toward their middles.
  const len = lerp(lengthOf(s), lengthOf(e), g);
  const m0 = mid(s);
  const m1 = mid(e);
  const c = { x: lerp(m0.x, m1.x, g), y: lerp(m0.y, m1.y, up) };
  const half = { x: (dir.x * len) / 2, y: (dir.y * len) / 2 };
  const pose = { a: { x: c.x - half.x, y: c.y - half.y }, b: { x: c.x + half.x, y: c.y + half.y }, width };
  return th ? { ...pose, b: turn(pose.b, pose.a, th) } : pose;
}

/**
 * The leg past its run: leaving at the speed the pen drew the run out at,
 * landing soft on the point (a cubic Hermite from that speed to rest).
 */
export function legAt(lt: number): number {
  if (lt < PEN[0]) return 0;
  const s = progress(PEN[0], PEN[1], lt);
  const run = lengthOf(TARGET[3]);
  // penLength's end speed, px a second.
  const v0 = (2 * (1 - PEN_LEAST) * run) / ((1 - PEN_DRAW) * (LAND[3] - LIFT[3]));
  const rest = (LOGO_STROKES[STROKE.kLeg].len - KLEG_RUN) * logoScale(LOGO_END);
  // Above 3 the Hermite would overshoot the point.
  const m0 = Math.min(2.8, (v0 * (PEN[1] - PEN[0])) / rest);
  const h = (s * s * s - 2 * s * s + s) * m0 + (-2 * s * s * s + 3 * s * s);
  return lerp(RUN_P, 1, h);
}

/** The shoulder springs out of the stem, fast then soft. */
export const shoulderAt = (lt: number): number => outCubic(progress(PEN[0], PEN[1], lt));
