// The config scene's flights (storyboard §6.2): seven names plucked out of
// the rivers fly into the hk.pkl card, one per sixteenth. A name in flight
// must never cross a line already drawn, a name being picked, or another
// flying name, and must stay on the stage: inside the side margin and above
// the captions' band. And the rivers, washing in and swinging into their
// columns, must not print over each other anywhere on the frame. Checked on
// the scene's own geometry (PROBE), with each name as its box on the mono
// grid, turned as it is drawn.

import assert from "node:assert/strict";
import { test } from "node:test";
import { BEAT } from "../bible";
import { HKPKL_LINES, monoWidth } from "../kit/card";
import { PROBE } from "../scenes/config";
import { maskAt, namePose } from "../scenes/config-rivers";

interface Box {
  x: number;
  y: number;
  a: number;
  x0: number;
  x1: number;
  y0: number;
  y1: number;
}

/** A name's box about its visual centre: the grid's width, ascenders to descenders. */
function nameBox(text: string, p: { x: number; y: number; a: number }, size: number): Box {
  const hw = (text.length * 0.6 * size) / 2;
  return { x: p.x, y: p.y, a: p.a, x0: -hw, x1: hw, y0: -0.4 * size, y1: (/[gjpqy_]/.test(text) ? 0.55 : 0.37) * size };
}

function corners(b: Box): { x: number; y: number }[] {
  const c = Math.cos(b.a);
  const s = Math.sin(b.a);
  return [
    [b.x0, b.y0],
    [b.x1, b.y0],
    [b.x1, b.y1],
    [b.x0, b.y1],
  ].map(([u, v]) => ({ x: b.x + u * c - v * s, y: b.y + u * s + v * c }));
}

/** Whether two turned boxes come within `gap` px of each other (separating axes). */
function touch(a: Box, b: Box, gap: number): boolean {
  const ca = corners(a);
  const cb = corners(b);
  for (const q of [ca, cb]) {
    for (let i = 0; i < 2; i++) {
      const nx = -(q[i + 1].y - q[i].y);
      const ny = q[i + 1].x - q[i].x;
      const l = Math.hypot(nx, ny);
      const pa = ca.map((p) => (p.x * nx + p.y * ny) / l);
      const pb = cb.map((p) => (p.x * nx + p.y * ny) / l);
      if (Math.max(...pa) + gap < Math.min(...pb) || Math.max(...pb) + gap < Math.min(...pa)) return false;
    }
  }
  return true;
}

const L = PROBE.layout;
const TEXT = HKPKL_LINES.map((l) => (typeof l === "string" ? l : ""));

/** A drawn row's glyphs: from its first non-space column to its end, cap height to descender. */
function rowBox(row: number): Box {
  const text = TEXT[row];
  const y = L.baseline(row);
  if (!text) {
    // The folded pill.
    const w = monoWidth("amends … import …", 28) + 32;
    return { x: 0, y: 0, a: 0, x0: L.x0, x1: L.x0 + w, y0: y - 10 - 19, y1: y - 10 + 19 };
  }
  return { x: 0, y: 0, a: 0, x0: L.col(text.length - text.trimStart().length), x1: L.col(text.length), y0: y - 27, y1: y + 8 };
}

/** The rows drawn at `t`, as far as a flight could meet them: from their landing (or the card's arrival). */
function rowsAt(t: number): { row: number; box: Box }[] {
  const rows: number[] = [];
  if (t >= 4 * BEAT) rows.push(0, 1);
  for (const st of PROBE.steps) if (t >= st.land) rows.push(st.line);
  if (t >= PROBE.clipIn[0]) rows.push(5, 6);
  if (t >= PROBE.stampFrom) rows.push(11);
  return rows.map((row) => ({ row, box: rowBox(row) }));
}

function bounds(b: Box) {
  const c = corners(b);
  return { x0: Math.min(...c.map((p) => p.x)), y1: Math.max(...c.map((p) => p.y)) };
}

test("no plucked name crosses a drawn line, a picked name or another in flight, and each stays on the stage", () => {
  const problems = new Set<string>();
  const first = PROBE.steps[0].takeoff - BEAT;
  const last = PROBE.steps[PROBE.steps.length - 1].land;
  for (let t = first; t <= last; t += 1 / 480) {
    const carried = PROBE.carried(t)
      .filter((c) => c.kind !== "pop")
      .map((c) => ({ ...c, box: nameBox(c.text, c.look.pose, c.look.size) }));
    for (const c of carried) {
      const own = PROBE.steps.find((s) => s.builtin === c.text)?.line;
      for (const r of rowsAt(t)) {
        if (r.row !== own && touch(c.box, r.box, 3)) problems.add(`${c.kind} ${c.text} crosses row ${r.row} at b${(t / BEAT).toFixed(3)}`);
      }
      if (c.kind !== "flight") continue;
      const b = bounds(c.box);
      if (b.x0 < 160) problems.add(`${c.text} leaves the side margin (x ${b.x0.toFixed(0)}) at b${(t / BEAT).toFixed(3)}`);
      if (b.y1 > 736) problems.add(`${c.text} reaches the captions' band (y ${b.y1.toFixed(0)}) at b${(t / BEAT).toFixed(3)}`);
    }
    for (let i = 0; i < carried.length; i++) {
      for (let j = i + 1; j < carried.length; j++) {
        if (touch(carried[i].box, carried[j].box, 2)) problems.add(`${carried[i].text} meets ${carried[j].text} at b${(t / BEAT).toFixed(3)}`);
      }
    }
  }
  assert.deepEqual([...problems].slice(0, 12), []);
});

test("each flight reaches its row, upright, left of the card's text, and lands on its Builtins reference", () => {
  const edge = L.col(2);
  for (const st of PROBE.steps) {
    const path = PROBE.paths[st.i];
    const to = { x: L.col(st.col) + (st.builtin.length * L.advance) / 2, y: L.baseline(st.line) - 0.34 * 34 };
    assert.ok(Math.abs(path.gate.y - to.y) < 1e-9, `${st.builtin}: gate on its row`);
    // Just before landing it is on its mark, at the card's size, upright.
    const k = PROBE.carried(st.land - 1e-6).find((c) => c.kind === "flight" && c.text === st.builtin);
    assert.ok(k, st.builtin);
    assert.ok(Math.abs(k.look.pose.x - to.x) < 0.5 && Math.abs(k.look.pose.y - to.y) < 0.5, `${st.builtin} lands on its reference`);
    assert.ok(Math.abs(k.look.size - 34) < 0.05 && Math.abs(k.look.pose.a) < 1e-3, `${st.builtin} lands at 34 px, upright`);
    // Right of its gate it is level with its row and keeps its swell to 4 px.
    for (let t = st.takeoff; t < st.land; t += 1 / 960) {
      const c = PROBE.carried(t).find((x) => x.kind === "flight" && x.text === st.builtin);
      if (!c) continue;
      const b = nameBox(c.text, c.look.pose, c.look.size);
      const right = Math.max(...corners(b).map((p) => p.x));
      if (right < edge - 4) continue;
      assert.ok(Math.abs(c.look.pose.y - to.y) < 0.5 && Math.abs(c.look.pose.a) < 1e-3, `${st.builtin} is level with its row over the text at b${(t / BEAT).toFixed(3)}`);
      assert.ok(c.look.size <= 38 + 1e-9, `${st.builtin} swells ${c.look.size.toFixed(1)} px over the text`);
    }
  }
});

test("the rivers never print over each other, washing in or swinging into their columns", () => {
  const crossings: string[] = [];
  // From the first wash to the columns, every visible name of each river
  // against the others', across the whole frame, bleeds included.
  for (let t = 0.5 * BEAT; t <= 4.75 * BEAT; t += 1 / 120) {
    const vis: { river: number; box: Box; text: string }[] = [];
    for (const nm of PROBE.names) {
      const { pose, alpha } = namePose(nm, t);
      if (alpha * maskAt(pose.y) < 0.06) continue;
      const box = nameBox(nm.text, pose, nm.size);
      const xs = corners(box).map((p) => p.x);
      if (Math.max(...xs) < 0 || Math.min(...xs) > 1920) continue;
      vis.push({ river: nm.river, box, text: nm.text });
    }
    for (let i = 0; i < vis.length; i++) {
      for (let j = i + 1; j < vis.length; j++) {
        // The boxes are generous (every name gets ascenders and descenders), so a graze of a pixel is not a crossing.
        if (vis[i].river !== vis[j].river && touch(vis[i].box, vis[j].box, -1)) crossings.push(`${vis[i].text} × ${vis[j].text} at b${(t / BEAT).toFixed(3)}`);
      }
    }
  }
  assert.deepEqual(crossings.slice(0, 12), []);
});
