// The morph's flights (storyboard §6.10, scenes/morph-pose.ts), checked on
// the geometry every 1/120 s: the k's arm comes up by the k's stem as a
// diagonal and docks on it from the right, never through it (no "L" and no
// tail poking out of the stem's far side); the stems swing no more than a
// hair past upright on their landings; and every bar is exactly on its
// stroke when the logo kit takes it over.

import assert from "node:assert/strict";
import { test } from "node:test";
import { BEAT } from "../bible";
import type { Pt } from "../kit/motion";
import { LAND, LIFT, SWAP } from "../scenes/morph-cues";
import { angleOf, type Pose, poseAt, SETTLE, TARGET } from "../scenes/morph-pose";

const STEM = 1;
const ARM = 2;
const DEG = 180 / Math.PI;

const sub = (p: Pt, q: Pt): Pt => ({ x: p.x - q.x, y: p.y - q.y });
const dot = (p: Pt, q: Pt): number => p.x * q.x + p.y * q.y;

function pointToSegment(p: Pt, a: Pt, b: Pt): number {
  const ab = sub(b, a);
  const t = Math.max(0, Math.min(1, dot(sub(p, a), ab) / dot(ab, ab)));
  return Math.hypot(p.x - a.x - ab.x * t, p.y - a.y - ab.y * t);
}

/** The clear space between two capsules' ink, px (negative where they overlap). */
function gap(s: Pose, q: Pose): number {
  const cross = (o: Pt, p: Pt, r: Pt) => (p.x - o.x) * (r.y - o.y) - (p.y - o.y) * (r.x - o.x);
  const crossing = cross(s.a, s.b, q.a) * cross(s.a, s.b, q.b) < 0 && cross(q.a, q.b, s.a) * cross(q.a, q.b, s.b) < 0;
  const d = crossing ? 0 : Math.min(pointToSegment(q.a, s.a, s.b), pointToSegment(q.b, s.a, s.b), pointToSegment(s.a, q.a, q.b), pointToSegment(s.b, q.a, q.b));
  return d - (s.width + q.width) / 2;
}

/** How far the arm's ink reaches past the stem's far (left) edge, px, where it is level with the stem. */
function poke(stem: Pose, arm: Pose): number {
  const u = sub(stem.b, stem.a);
  const len = Math.hypot(u.x, u.y);
  const along = { x: u.x / len, y: u.y / len };
  // The stem's normal toward its right, where the arm belongs.
  const right = { x: -along.y, y: along.x };
  const sign = dot(sub(arm.b, stem.a), right) >= 0 ? 1 : -1;
  let worst = -Infinity;
  for (const q of [arm.a, arm.b]) {
    const k = dot(sub(q, stem.a), along);
    if (k < -stem.width / 2 || k > len + stem.width / 2) continue;
    worst = Math.max(worst, arm.width / 2 - sign * dot(sub(q, stem.a), right) - stem.width / 2);
  }
  return worst;
}

/** Every 1/120 s from `a` to `b`, local seconds. */
function* frames(a: number, b: number): Generator<number> {
  for (let n = Math.ceil(a * 120); n / 120 <= b; n++) yield n / 120;
}

/** How steeply the arm rises, degrees (its stroke rises at 32.9°). */
const rising = (p: Pose): number => -angleOf(p) * DEG;

test("the k's arm docks on the k's stem from the right and never pokes out of its far side", () => {
  const root = TARGET[ARM].a;
  for (const lt of frames(LIFT[ARM], SETTLE[ARM])) {
    const stem = poseAt(STEM, lt);
    const arm = poseAt(ARM, lt);
    const at = `b${(lt / BEAT).toFixed(3)}`;
    if (gap(stem, arm) < 0) assert.ok(poke(stem, arm) <= 0.25, `${at}: the arm's ink is ${poke(stem, arm).toFixed(2)} px out of the stem's far side`);
    // It slides in under the stem's foot; once its tail is level with the
    // stem it is on the stem's right, and its ink never reaches past the
    // stem's left edge, where the landed arm's does.
    const foot = stem.a.y > stem.b.y ? stem.a : stem.b;
    if (arm.a.y - arm.width / 2 < foot.y + stem.width / 2) {
      assert.ok(arm.a.x >= root.x - 1e-6, `${at}: the arm's tail at x ${arm.a.x.toFixed(1)} is left of its root`);
      assert.ok(arm.a.x - arm.width / 2 >= root.x - TARGET[STEM].width / 2 - 0.25, `${at}: the arm's ink is left of the stem's`);
    }
  }
});

test("the k's arm is a diagonal whenever it is near the stem, so the two never read as an L", () => {
  let near = 0;
  for (const lt of frames(LIFT[ARM], LAND[ARM])) {
    const stem = poseAt(STEM, lt);
    const arm = poseAt(ARM, lt);
    const g = gap(stem, arm);
    const at = `b${(lt / BEAT).toFixed(3)}`;
    if (g < 40) {
      near++;
      assert.ok(rising(arm) >= 24, `${at}: the arm rises at ${rising(arm).toFixed(1)}°, ${g.toFixed(0)} px from the stem`);
    }
    // An L: the stem near upright and the arm near flat, close by.
    if (g < 60 && Math.abs(rising(stem) - 90) < 20) assert.ok(rising(arm) >= 25, `${at}: an L (the arm at ${rising(arm).toFixed(1)}°)`);
  }
  assert.ok(near > 0, "the arm does come up by the stem");
});

test("the stems swing no more than 2° past upright on their landings, and are still from SETTLE", () => {
  for (const i of [0, 1]) {
    let most = 0;
    for (const lt of frames(LIFT[i], SETTLE[i])) most = Math.max(most, Math.abs(rising(poseAt(i, lt)) - 90) * (lt >= LAND[i] - 0.1 * BEAT ? 1 : 0));
    assert.ok(most <= 2, `stem ${i} swings ${most.toFixed(2)}° past upright`);
  }
});

test("each bar is exactly on its stroke when the logo kit takes it over", () => {
  for (let i = 0; i < 4; i++) {
    const lt = i === 3 ? SWAP : SETTLE[i];
    assert.deepEqual(poseAt(i, lt), TARGET[i], `bar ${i} at b${(lt / BEAT).toFixed(2)}`);
  }
});
