// The chapters track served next to the video is generated from SECTIONS.
// When the sections change, rewrite it with
// `UPDATE_CHAPTERS=1 aube run test:showreel` from docs/ and commit it.

import assert from "node:assert/strict";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { CHAPTERS, chaptersVtt, DURATION, SECTIONS } from "../timeline";

/** docs/, found from the working directory, since the tests run from a bundle. */
function docsDir(): string {
  for (let dir = process.cwd(); ; dir = dirname(dir)) {
    if (existsSync(join(dir, ".vitepress/theme/showreel"))) return dir;
    if (existsSync(join(dir, "docs/.vitepress/theme/showreel"))) return join(dir, "docs");
    if (dirname(dir) === dir) throw new Error("run the tests from inside the repository");
  }
}

test("docs/public/showreel-chapters.vtt is the chapters track SECTIONS generates", () => {
  const file = join(docsDir(), "public/showreel-chapters.vtt");
  if (process.env.UPDATE_CHAPTERS) writeFileSync(file, chaptersVtt());
  const served = existsSync(file) ? readFileSync(file, "utf8") : "";
  assert.equal(served, chaptersVtt(), "the chapters track is stale: run `UPDATE_CHAPTERS=1 aube run test:showreel` in docs/");
});

test("the chapters track has one cue per section, end to end", () => {
  const blocks = chaptersVtt().trimEnd().split("\n\n");
  assert.equal(blocks[0], "WEBVTT");
  const cues = blocks.slice(1).map((b) => b.split("\n"));
  assert.equal(cues.length, SECTIONS.length);
  const time = (s: string) => {
    const [h, m, sec] = s.split(":");
    return Number(h) * 3600 + Number(m) * 60 + Number(sec);
  };
  cues.forEach(([id, span, label], i) => {
    const [a, b] = span.split(" --> ").map(time);
    assert.equal(id, SECTIONS[i].id);
    assert.equal(label, SECTIONS[i].label);
    assert.equal(a, CHAPTERS[i].start);
    assert.equal(b, CHAPTERS[i].end);
    assert.match(span, /^\d\d:\d\d:\d\d\.\d{3} --> \d\d:\d\d:\d\d\.\d{3}$/);
  });
  assert.equal(time(cues[cues.length - 1][1].split(" --> ")[1]), DURATION);
});
