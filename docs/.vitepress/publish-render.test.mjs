import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { publishRender } from "./publish-render.mjs";

/** A staged render and the published pair it replaces, in a fresh directory. */
function setup() {
  const dir = mkdtempSync(join(tmpdir(), "publish-render-"));
  const at = (name) => join(dir, name);
  writeFileSync(at("new.mp4"), "new video");
  writeFileSync(at("new.jpg"), "new poster");
  writeFileSync(at("video.mp4"), "old video");
  writeFileSync(at("poster.jpg"), "old poster");
  const pair = { video: { from: at("new.mp4"), to: at("video.mp4") }, poster: { from: at("new.jpg"), to: at("poster.jpg") } };
  return { dir, at, pair };
}

test("a render publishes its video and poster over the old pair", () => {
  const { dir, at, pair } = setup();
  try {
    publishRender(pair);
    assert.equal(readFileSync(at("video.mp4"), "utf8"), "new video");
    assert.equal(readFileSync(at("poster.jpg"), "utf8"), "new poster");
    assert.ok(!existsSync(at("new.mp4")) && !existsSync(at("new.jpg")));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("when its poster cannot go up, the render takes its video down again", () => {
  const { dir, at, pair } = setup();
  try {
    // A directory where the poster goes: renaming a file over it fails.
    rmSync(at("poster.jpg"));
    mkdirSync(join(at("poster.jpg"), "blocker"), { recursive: true });
    assert.throws(() => publishRender(pair));
    assert.ok(!existsSync(at("video.mp4")), "the new video is still published without its poster");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("when its video cannot go up, the old pair stays as it was", () => {
  const { dir, at, pair } = setup();
  try {
    rmSync(at("new.mp4"));
    assert.throws(() => publishRender(pair));
    assert.equal(readFileSync(at("video.mp4"), "utf8"), "old video");
    assert.equal(readFileSync(at("poster.jpg"), "utf8"), "old poster");
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
