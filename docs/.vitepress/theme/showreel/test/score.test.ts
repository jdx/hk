import assert from "node:assert/strict";
import { test } from "node:test";
import { playScore } from "../audio";
import type { ReelFacts } from "../facts";
import { GAP } from "../score/morph";
import { DURATION, SECTIONS, sec } from "../timeline";
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
      // Sources start a few milliseconds early, ahead of the compressor's lookahead.
      assert.ok(start > -0.01 && start < DURATION, `${variant}: ${target} starts at ${start}`);
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
      else assert.ok(start >= resolve - 0.01, `${variant}: ${target} starts at ${start}, in the breath`);
    }
  }
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
