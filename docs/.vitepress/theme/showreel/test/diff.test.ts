// The diffs on everywhere's two cards are ruff's own (README in
// test/captures): under hk check, ruff-format's diff of src/main.py as the
// stash scene shows it staged (ruff-format.diff.txt); under hk fix, its
// diff once ruff, which it depends on, has removed the unused import
// (ruff-format.after-ruff.diff.txt). Each card's rows are a run of its
// capture's lines, verbatim, and applying the fix diff gives the file the
// commit got.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { MAIN_PY_FIXED, MAIN_PY_STAGED, RUFF_FORMAT_DIFF, RUFF_FORMAT_FIX_DIFF } from "../kit/card";
import { SHOWREEL } from "./repo";

const capture = (name: string) => readFileSync(join(SHOWREEL, "test/captures", name), "utf8").replace(/\n$/, "").split("\n");
const CHECK = capture("ruff-format.diff.txt");
const FIX = capture("ruff-format.after-ruff.diff.txt");

/**
 * A capture's one hunk: its header's line counts and its rows. The
 * captures are right-trimmed and their trailing blank rows dropped, as the
 * other captures are, so a context row that was a lone space is empty, and
 * the hunk's last rows are blank context the header still counts.
 */
function hunk(lines: readonly string[]): { old: number; new: number; rows: string[] } {
  assert.deepEqual(lines.slice(0, 2), ["--- src/main.py", "+++ src/main.py"]);
  const m = /^@@ -1,(\d+) \+1,(\d+) @@$/.exec(lines[2]);
  assert.ok(m, `one hunk from line 1: ${lines[2]}`);
  return { old: Number(m[1]), new: Number(m[2]), rows: lines.slice(3) };
}

/** One side of a hunk, `-` or `+`, with context, padded with the blank context rows the capture drops. */
function side(lines: readonly string[], sign: "-" | "+"): string[] {
  const h = hunk(lines);
  const count = sign === "-" ? h.old : h.new;
  const rows = h.rows.filter((r) => r === "" || r[0] === " " || r[0] === sign).map((r) => r.slice(1));
  assert.ok(rows.length <= count, `${rows.length} rows for a side of ${count}`);
  return [...rows, ...Array<string>(count - rows.length).fill("")];
}

test("each card's rows are the top of its capture's hunk, verbatim", () => {
  for (const [card, lines] of [
    [RUFF_FORMAT_DIFF, CHECK],
    [RUFF_FORMAT_FIX_DIFF, FIX],
  ] as const) {
    assert.deepEqual(card, hunk(lines).rows.slice(0, card.length));
    // Removals and additions both, so the card has something to strike and something to settle.
    assert.ok(card.some((l) => l.startsWith("-")) && card.some((l) => l.startsWith("+")));
  }
  // The same number of rows, so the two cards unfold on the same grid.
  assert.equal(RUFF_FORMAT_FIX_DIFF.length, RUFF_FORMAT_DIFF.length);
});

test("hk check's diff is of src/main.py as staged", () => {
  assert.deepEqual(side(CHECK, "-"), MAIN_PY_STAGED.slice(0, hunk(CHECK).old));
});

test("applied, hk check's diff formats the file but keeps the import ruff removes", () => {
  assert.deepEqual(side(CHECK, "+"), ["import os", "", "", ...MAIN_PY_FIXED.slice(0, hunk(CHECK).new - 3)]);
});

test("hk fix's diff is of the staged file once ruff has removed its unused import", () => {
  assert.equal(MAIN_PY_STAGED[0], "import os");
  assert.deepEqual(side(FIX, "-"), MAIN_PY_STAGED.slice(1, 1 + hunk(FIX).old));
});

test("applied, hk fix's diff gives the file the commit got, and so do the card's settled rows", () => {
  assert.deepEqual(side(FIX, "+"), MAIN_PY_FIXED.slice(0, hunk(FIX).new));
  // The card's rows once hk has applied it: the removals gone, the rest without diff's column.
  const settled = RUFF_FORMAT_FIX_DIFF.filter((l) => !l.startsWith("-")).map((l) => l.slice(1));
  assert.deepEqual(settled, MAIN_PY_FIXED.slice(0, settled.length));
});
