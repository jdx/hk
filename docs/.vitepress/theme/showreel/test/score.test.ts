import assert from "node:assert/strict";
import { test } from "node:test";
import { playScore } from "../audio";
import type { ReelFacts } from "../facts";
import { checkAll } from "../kit/screens";
import { LATENCY, X } from "../score/mix";
import { GAP } from "../score/morph";
import { raceRuns } from "../race-timing";
import { F0_EACH, F0_FIRST } from "../scenes/race";
import { arc, PARTS } from "../score";
import { BEAT, DURATION, SECTIONS, sec } from "../timeline";
import { MockContext } from "./mock-audio";
import { factsFor, type Variant } from "./published";

/** The context time reel time 0 plays at, as the MP4 renderer schedules it. */
const WHEN = 0.2;

/** The race's cues follow the facts, so every check runs under each variant the picture has. */
const VARIANTS: Variant[] = ["both", "one", "none"];

function render(from = 0, facts: ReelFacts | null = null): MockContext {
  const ac = new MockContext();
  playScore(ac.context, ac.destination as AudioNode, from, WHEN, facts);
  return ac;
}

/** Each source's start and stop, in reel seconds, for a score started at `from`. */
function spans(ac: MockContext, from = 0): { target: string; start: number; stop: number }[] {
  const stops = new Map<string, number>();
  for (const c of ac.calls) if (c.method === "stop") stops.set(c.target, c.args[0] as number);
  return ac.starts().map(({ target, t }) => ({ target, start: t - WHEN + from, stop: (stops.get(target) ?? Infinity) - WHEN + from }));
}

test("the score starts every source inside the reel, and sounds in every section", () => {
  for (const variant of VARIANTS) {
    const starts = spans(render(0, factsFor(variant)));
    for (const { target, start } of starts) {
      // Sources start a few milliseconds early, ahead of the master's compressor lookahead.
      assert.ok(start >= -LATENCY - 0.001 && start < DURATION, `${variant}: ${target} starts at ${start}`);
    }
    for (const { id } of SECTIONS) {
      const s = sec(id);
      assert.ok(
        starts.some(({ start }) => start >= s.start - 0.01 && start < s.end - 0.01),
        `${variant}: nothing sounds in ${id}`,
      );
    }
  }
});

test("nothing sounds in the breath before the end card's downbeat", () => {
  const resolve = sec("end").start;
  for (const variant of VARIANTS) {
    for (const { target, start, stop } of spans(render(0, factsFor(variant)))) {
      // A source stops 20 ms after its envelope has fallen silent.
      if (start < GAP) assert.ok(stop <= GAP + 0.03, `${variant}: ${target} (from ${start}) rings until ${stop}, into the breath at ${GAP}`);
      else assert.ok(start >= resolve - LATENCY - 0.001, `${variant}: ${target} starts at ${start}, in the breath`);
    }
  }
});

test("every automation curve runs forward in time, and every exponential ramp between positive values", () => {
  // Web Audio sorts a param's events by time, so a curve whose points run
  // backwards (an envelope's peak written after its end) plays as a
  // different curve, often silence, and an exponential ramp to or from 0
  // jumps instead of ramping. A value curve owns its whole span: Chromium
  // rejects any event that starts inside it.
  const TIMED = ["setValueAtTime", "linearRampToValueAtTime", "exponentialRampToValueAtTime", "setTargetAtTime", "setValueCurveAtTime"];
  const reel = (t: number) => (t - WHEN + LATENCY).toFixed(4);
  const bad: string[] = [];
  for (const variant of VARIANTS) {
    // Per param: when its last event ends (a curve's end, any other event's
    // time), and the value it leaves (NaN after a curve: the mock keeps only
    // a curve's length and sum).
    const last = new Map<string, number>();
    const value = new Map<string, number>();
    for (const c of render(0, factsFor(variant)).calls) {
      if (c.method === "value=") value.set(c.target, c.args[0] as number);
      if (c.method === "cancelScheduledValues") last.delete(c.target);
      if (!TIMED.includes(c.method)) continue;
      const [v, t, span] = c.args as [number, number, number];
      const curve = c.method === "setValueCurveAtTime";
      const before = last.get(c.target) ?? -Infinity;
      if (t < before - 1e-9) bad.push(`${variant}: ${c.target} ${c.method}(${curve ? "curve" : v}) at ${reel(t)} starts before ${reel(before)}, where its last event ends`);
      const from = value.get(c.target) ?? 0;
      if (c.method === "exponentialRampToValueAtTime" && !(v > 0 && from > 0)) bad.push(`${variant}: ${c.target} ramps exponentially from ${from} to ${v} at ${reel(t)}`);
      last.set(c.target, curve ? t + span : t);
      value.set(c.target, curve ? NaN : v);
    }
  }
  assert.deepEqual(bad, []);
});

test("the score schedules the same calls every time, from any start", () => {
  for (const variant of VARIANTS) {
    for (const from of [0, 4.2, 11, 37.3, 50.3, 57.9]) {
      assert.deepEqual(render(from, factsFor(variant)).calls, render(from, factsFor(variant)).calls, `${variant} from ${from}`);
    }
  }
});

test("a start mid-reel plays the same sounds, on the same samples, as playback from the top", () => {
  const onSample = (t: number) => Math.round(t * 48000 + 0.5);
  for (const variant of VARIANTS) {
    const facts = factsFor(variant);
    const whole = spans(render(0, facts)).map((s) => s.start);
    for (const from of [4.2, 23.9, 50.3]) {
      // Sustained sounds already under way enter mid-sound at `from`; everything after it matches exactly.
      const after = (list: number[]) => list.filter((t) => t >= from + 0.1).map(onSample).sort((a, b) => a - b);
      assert.deepEqual(after(spans(render(from, facts), from).map((s) => s.start)), after(whole), `${variant} from ${from}`);
    }
  }
});

test("each bar in the benchmark race stops on a sound, only when the facts back its race", () => {
  const race = sec("race");
  // Sources start LATENCY early, ahead of the master's compressors, and half a sample before their frame.
  const soundsAt = (starts: number[], t: number) => starts.some((s) => Math.abs(s + LATENCY - t) < 0.001);
  const stopsOf = (variant: Variant) => raceRuns(factsFor(variant)).flatMap((run) => Object.values(run.stops).map((b) => race.beat(b)));
  for (const variant of VARIANTS) {
    const starts = spans(render(0, factsFor(variant))).map((s) => s.start);
    const stops = stopsOf(variant);
    for (const t of stops) assert.ok(soundsAt(starts, t), `${variant}: nothing sounds as a bar stops at ${t.toFixed(4)}`);
    // Stops off the sixteenth grid, where nothing else is written, stay silent when the facts do not back their race.
    const offGrid = (t: number) => Math.abs(t / X - Math.round(t / X)) > 0.01;
    for (const t of stopsOf("both").filter((t) => !stops.includes(t) && offGrid(t))) {
      assert.ok(!soundsAt(starts, t), `${variant}: a sound at ${t.toFixed(4)}, where a race the facts do not back would stop`);
    }
  }
});

test("with no race, each ✔ row of hk's check run, and the 7/7, sounds on the frame it lands", () => {
  const race = sec("race");
  // Sources start LATENCY early, ahead of the master's compressors.
  const starts = spans(render(0, factsFor("none"))).map((s) => s.start + LATENCY);
  const soundsAt = (t: number) => starts.some((s) => s > t - 0.001 && s < t + 0.005);
  let frames = 0;
  checkAll.forEach((rows, f) => {
    if (!f) return;
    const before = new Set<string>(checkAll[f - 1]);
    const lands = rows.slice(1).some((r) => r.startsWith("✔ ") && !before.has(r)) || (rows[0].endsWith("7/7") && !checkAll[f - 1][0].endsWith("7/7"));
    if (!lands) return;
    frames++;
    const t = race.at(F0_FIRST + F0_EACH * f);
    assert.ok(soundsAt(t), `frame ${f} lands at ${t.toFixed(4)} with nothing sounding within 5 ms`);
  });
  assert.ok(frames >= 7, `only ${frames} frames land a ✔ row`);
});

test("the groove's fader holds each section's level and moves only over the half beat before a bar line", () => {
  const pts = arc();
  // The fader's value at reel time t: it only ever moves in straight lines.
  const at = (t: number): number => {
    let v = pts[0][1];
    for (let i = 1; i < pts.length; i++) {
      const [t0, v0] = pts[i - 1];
      const [t1, v1] = pts[i];
      if (t >= t1) v = v1;
      else if (t > t0) return v0 + ((v1 - v0) * (t - t0)) / (t1 - t0);
    }
    return v;
  };
  for (const { id } of SECTIONS) {
    const s = sec(id);
    const level = PARTS[id].level ?? 1;
    for (const t of [s.start, (s.start + s.end) / 2, s.end - BEAT / 2 - 0.001]) {
      assert.ok(Math.abs(at(t) - level) < 1e-9, `${id}: the fader is at ${at(t)} at ${t}, not its level ${level}`);
    }
  }
  // The arc: the groove enters softly, and the lanes and the race ride above the rest.
  const level = (id: keyof typeof PARTS) => PARTS[id].level ?? 1;
  assert.ok(level("config") < 1 && level("lanes") > 1 && level("race") > 1);
});
