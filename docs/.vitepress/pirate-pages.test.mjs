// The checks that keep each pirate page (docs/pirate/<page>.md) in step with
// its English page: what counts as a change, and what a variant must keep.

import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { after, test } from "node:test";
import { compareFiles, expandIncludes, normalize, stampText } from "./pirate-pages.mjs";

const dir = mkdtempSync(join(tmpdir(), "pirate-pages-"));
after(() => rmSync(dir, { recursive: true, force: true }));
let n = 0;
/** Writes an English page and its variant, and returns what the check says of them. */
async function compare(english, pirate) {
  n++;
  const a = join(dir, `english-${n}.md`);
  const b = join(dir, `pirate-${n}.md`);
  writeFileSync(a, english);
  writeFileSync(b, pirate);
  return compareFiles(a, b);
}

const ENGLISH = `---
description: Configure hooks.
outline: deep
---

# Hooks

Run \`hk check\` before you commit. See [stashing](/hooks#stashing) and [mise](https://mise.jdx.dev).

## Check and fix

::: tip Make the linters available
Install them first.
:::

| Command    | Does            |
| ---------- | --------------- |
| \`hk check\` | Checks files    |
| \`hk fix\`   | Fixes files     |

\`\`\`pkl
steps { ["prettier"] = Builtins.prettier }
\`\`\`

### Stashing

Unstaged work is kept.
`;

const PIRATE = `---
description: Rig yer hooks.
outline: deep
sourceHash: 123456789012
---

# Hooks o' the ship

Run \`hk check\` afore ye set sail. See [stowin' the hold](/hooks#stashing) and [the quartermaster](https://mise.jdx.dev).

## Inspect and mend {#check-and-fix}

::: tip Muster yer lookouts
Bring 'em aboard first.
:::

| The call   | What it does         |
| ---------- | -------------------- |
| \`hk check\` | Inspects the cargo   |
| \`hk fix\`   | Mends the canvas     |

\`\`\`pkl
steps { ["prettier"] = Builtins.prettier }
\`\`\`

### Stowing the hold {#stashing}

Cargo left on the dock stays safe.
`;

test("a variant with the English page's structure and its own words passes", async () => {
  assert.deepEqual(await compare(ENGLISH, PIRATE), []);
});

test("a heading without the English anchor is reported", async () => {
  const problems = await compare(ENGLISH, PIRATE.replace("## Inspect and mend {#check-and-fix}", "## Inspect and mend"));
  assert.equal(problems.length, 1);
  assert.match(problems[0], /sections differ/);
  assert.match(problems[0], /h2#check-and-fix/);
});

test("a heading out of order or at another level is reported", async () => {
  const problems = await compare(ENGLISH, PIRATE.replace("### Stowing the hold {#stashing}", "## Stowing the hold {#stashing}"));
  assert.match(problems.join("\n"), /sections differ/);
});

test("a code block must be copied byte for byte", async () => {
  const problems = await compare(ENGLISH, PIRATE.replace('["prettier"] = Builtins.prettier', '["prettier"] = Builtins.prettier // arr'));
  assert.match(problems.join("\n"), /code block 1 differs/);
  const retagged = await compare(ENGLISH, PIRATE.replace("```pkl", "```"));
  assert.match(retagged.join("\n"), /code block 1 differs/);
});

test("links are kept exactly: none dropped, none added", async () => {
  const dropped = await compare(ENGLISH, PIRATE.replace("[the quartermaster](https://mise.jdx.dev)", "the quartermaster"));
  assert.match(dropped.join("\n"), /drops links[\s\S]*mise\.jdx\.dev/);
  const added = await compare(ENGLISH, PIRATE.replace("Bring 'em aboard first.", "Bring 'em aboard first, [matey](/glossary)."));
  assert.match(added.join("\n"), /adds links[\s\S]*\/glossary/);
  const moved = await compare(ENGLISH, PIRATE.replace("(/hooks#stashing)", "(/pirate/hooks#stashing)"));
  assert.ok(moved.some((p) => /drops links/.test(p)) && moved.some((p) => /adds links/.test(p)));
});

test("inline code is kept, and more of it is allowed", async () => {
  const dropped = await compare(ENGLISH, PIRATE.replace("| `hk fix`   | Mends the canvas     |", "| hk fix     | Mends the canvas     |"));
  assert.match(dropped.join("\n"), /drops inline code[\s\S]*"hk fix"/);
  assert.deepEqual(await compare(ENGLISH, PIRATE.replace("Cargo left on the dock stays safe.", "Cargo left on the dock stays safe, even with `hk check`.")), []);
});

test("containers and tables keep their kind and shape", async () => {
  const container = await compare(ENGLISH, PIRATE.replace("::: tip Muster yer lookouts", "::: warning Muster yer lookouts"));
  assert.match(container.join("\n"), /containers differ/);
  const table = await compare(ENGLISH, PIRATE.replace("| `hk fix`   | Mends the canvas     |\n", ""));
  assert.match(table.join("\n"), /tables differ/);
});

test("front matter: its own words, every other key copied, and a recorded hash", async () => {
  const outline = await compare(ENGLISH, PIRATE.replace("outline: deep\n", "outline: [2, 3]\n"));
  assert.match(outline.join("\n"), /front matter `outline`/);
  const noDescription = await compare(ENGLISH, PIRATE.replace("description: Rig yer hooks.\n", ""));
  assert.match(noDescription.join("\n"), /needs its own `description`/);
  const noHash = await compare(ENGLISH, PIRATE.replace("sourceHash: 123456789012\n", ""));
  assert.match(noHash.join("\n"), /no `sourceHash`/);
});

test("Vue interpolation in prose is reported", async () => {
  const problems = await compare(ENGLISH, PIRATE.replace("Cargo left on the dock stays safe.", "Cargo {{ stays }} safe."));
  assert.match(problems.join("\n"), /\{\{/);
});

test("components and images are kept", async () => {
  const problems = await compare(`${ENGLISH}\n<ShantyVideo />\n`, PIRATE);
  assert.match(problems.join("\n"), /components differ/);
});

test("a release's version bump does not change what a variant was written from", () => {
  const url = (v) => `amends "package://github.com/jdx/hk/releases/download/v${v}/hk@${v}#/Config.pkl"`;
  assert.equal(normalize(url("2.4.0")), normalize(url("2.10.1")));
  assert.equal(normalize(url("2.4.0")), normalize(url("2.5.0-rc.1")));
  assert.notEqual(normalize(`${url("2.4.0")}\nold`), normalize(`${url("2.4.0")}\nnew`));
  assert.equal(normalize("a\r\nb"), "a\nb");
  assert.equal(normalize("**Version:** 2.4.0\n"), normalize("**Version:** 2.5.0-rc.1\n"));
});

test("stamping writes the hash into the front matter, adding it or replacing it", () => {
  assert.equal(stampText("---\ntitle: x\n---\n\n# X\n", "abc"), "---\ntitle: x\nsourceHash: abc\n---\n\n# X\n");
  assert.equal(stampText("---\ntitle: x\nsourceHash: 111\n---\n\n# X\n", "abc"), "---\ntitle: x\nsourceHash: abc\n---\n\n# X\n");
  assert.equal(stampText("# X\n", "abc"), "---\nsourceHash: abc\n---\n\n# X\n");
  // Dollar signs in a page are text, not replacement patterns.
  const dollars = "---\ndescription: costs $$ and $& and $'\n---\n\nrun `$'x'`\n";
  assert.equal(stampText(dollars, "abc"), "---\ndescription: costs $$ and $& and $'\nsourceHash: abc\n---\n\nrun `$'x'`\n");
  assert.equal(stampText(stampText(dollars, "abc"), "def"), "---\ndescription: costs $$ and $& and $'\nsourceHash: def\n---\n\nrun `$'x'`\n");
});

test("includes are expanded without their front matter, as VitePress does", () => {
  mkdirSync(join(dir, "gen"), { recursive: true });
  writeFileSync(join(dir, "gen", "part.md"), "---\ntitle: part\n---\n## Part\n");
  const page = join(dir, "page.md");
  assert.equal(expandIncludes("A\n<!--@include: ./gen/part.md-->\nB", page), "A\n## Part\n\nB");
  assert.equal(expandIncludes("<!--@include: ./gen/missing.md-->", page), "<!--@include: ./gen/missing.md-->");
});
