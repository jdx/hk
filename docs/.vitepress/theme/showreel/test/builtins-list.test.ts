// The config scene's rivers stream today's builtin names as texture. Every
// name must still be a builtin, so the reel never shows one hk has dropped;
// a builtin added since is simply not in the rivers, which fails nothing.

import assert from "node:assert/strict";
import { readdirSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { BUILTIN_POPS, BUILTINS } from "../kit/builtins-list";
import { HKPKL_LINES, HKPKL_STEPS } from "../kit/card";
import { REPO } from "./repo";

// The identifier hk.pkl uses for pkl/builtins/<stem>.pkl, as
// scripts/gen_builtins.py makes it: the stem with every `-` turned into `_`.
// Mapped from the files, since the reverse is ambiguous.
const identifiers = new Map(
  readdirSync(join(REPO, "pkl/builtins"))
    .filter((f) => f.endsWith(".pkl"))
    .map((f) => [f.slice(0, -4).replaceAll("-", "_"), f] as const),
);

test("every name in the rivers is a builtin today", () => {
  const missing = BUILTINS.filter((name) => !identifiers.has(name));
  assert.deepEqual(missing, [], `not in pkl/builtins: ${missing.join(", ")}`);
});

test("the list is sorted, unique, frozen, and names as Pkl spells them", () => {
  assert.ok(Object.isFrozen(BUILTINS));
  assert.deepEqual([...BUILTINS], [...new Set(BUILTINS)].sort());
  for (const name of BUILTINS) assert.match(name, /^[a-z][a-z0-9_]*$/, name);
  // The one stem with a dash appears under its identifier.
  assert.ok(BUILTINS.includes("editorconfig_checker"));
  assert.equal(identifiers.get("editorconfig_checker"), "editorconfig-checker.pkl");
});

test("the named builtins are in the rivers: the two pops and the seven steps", () => {
  for (const name of BUILTIN_POPS) assert.ok(BUILTINS.includes(name), name);
  for (const s of HKPKL_STEPS) assert.ok(BUILTINS.includes(s.builtin), s.builtin);
  // And every Builtins.x the hk.pkl card shows.
  const used = HKPKL_LINES.flatMap((l) => [...(typeof l === "string" ? l : "").matchAll(/Builtins\.(\w+)/g)].map((m) => m[1]));
  assert.equal(used.length, 7);
  for (const name of used) assert.ok(BUILTINS.includes(name), name);
});
