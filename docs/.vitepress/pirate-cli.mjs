// The pirate CLI reference. docs/cli/** is generated from hk's usage spec
// (`mise run render:usage`), and so is its pirate variant, docs/pirate/cli/**:
// this script rebuilds it from the English pages and the crew's words for
// them in docs/pirate/cli.json, and `mise run render:usage` runs it after the
// English pages. Each line of an English page becomes the same line of its
// pirate page: code blocks, commands, flags and links as they are, and prose
// in the crew's words wherever cli.json has them. A line cli.json does not
// know yet (a flag added or reworded since) stays in English, so the pirate
// reference always matches the CLI and never goes stale.
//
//   node .vitepress/pirate-cli.mjs            # rebuild docs/pirate/cli
//   node .vitepress/pirate-cli.mjs --missing  # the English lines with no pirate words yet
//
// cli.json holds, all keyed by the English text as the generated pages have it:
//
//   "labels": line prefixes swapped wherever they start a line ("- **Usage:** ")
//   "lines":  whole lines, and table cells, in the crew's words, for every page
//   "pages":  the same, for one page only ("util.md"), where it needs its own words
//
// A heading's pirate words are given without an anchor: the page keeps the
// English heading's anchor, which this script adds. docs/pirate/STYLE.md has
// the voice and the lexicon. Edit cli.json, never the generated pages.

import { existsSync, globSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, posix, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { DOCS, hashText, PIRATE_DIR } from "./pirate-pages.mjs";

export const WORDS = join(DOCS, PIRATE_DIR, "cli.json");

/** VitePress's heading slugs (its `slugify`), so a pirate heading carries the English one's anchor. */
export function slugify(text) {
  return text
    .normalize("NFKD")
    .replace(/[\u0300-\u036F]/g, "")
    .replace(/[\u0000-\u001f]/g, "")
    .replace(/[\s~`!@#$%^&*()\-_+=[\]{}|\\;:"'“”‘’<>,.?/]+/g, "-")
    .replace(/-{2,}/g, "-")
    .replace(/^-+|-+$/g, "")
    .replace(/^(\d)/, "_$1")
    .toLowerCase();
}

/**
 * A heading's text as VitePress slugs it: only its text and code, without
 * Markdown or HTML. Code is set aside first, so its `_` and `*` stay, and an
 * underscore inside a word (`check_diff`) is not emphasis.
 */
function headingText(markdown) {
  const code = [];
  return markdown
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/`([^`]*)`/g, (_, text) => `\u0000${code.push(text) - 1}\u0000`)
    .replace(/(\*\*|\*)(.+?)\1/g, "$2")
    .replace(/(?<![A-Za-z0-9])(__|_)(.+?)\1(?![A-Za-z0-9])/g, "$2")
    .replace(/<[^>]*>/g, "")
    .replace(/\u0000(\d+)\u0000/g, (_, n) => code[n]);
}

/** Whether a line has words for the crew to say, beyond code, links and punctuation. */
export function hasProse(line) {
  return line
    .replace(/`[^`]*`/g, " ")
    .replace(/\]\([^)]*\)/g, "]")
    .replace(/<[^>]*>/g, " ")
    .replace(/[^A-Za-z]+/g, " ")
    .trim()
    .split(" ")
    .some((word) => word.length > 1);
}

const TABLE_RE = /^\|.*\|$/;
const RULE_RE = /^\|(?:\s*:?-+:?\s*\|)+$/;
/** A row's cells: split at pipes, but not at escaped ones (`\|`), which stay in their cell. */
const cellsOf = (line) => line.slice(1, -1).split(/(?<!\\)\|/);
const alignOf = (cell) => {
  const rule = cell.trim();
  if (rule.startsWith(":") && rule.endsWith(":")) return "center";
  return rule.endsWith(":") ? "right" : rule.startsWith(":") ? "left" : null;
};

/**
 * A table's rows padded as prettier pads them, so the page is already
 * formatted. The rule row is `{ align }`, one alignment (or null) per column.
 */
function formatTable(rows) {
  const width = (cell) => [...cell].length;
  const widths = [];
  for (const row of rows) if (Array.isArray(row)) row.forEach((cell, i) => (widths[i] = Math.max(widths[i] ?? 3, width(cell))));
  const align = rows.find((row) => !Array.isArray(row))?.align ?? [];
  const pad = (cell, i) => {
    const room = widths[i] - width(cell);
    if (align[i] === "right") return " ".repeat(room) + cell;
    if (align[i] === "center") return " ".repeat(Math.floor(room / 2)) + cell + " ".repeat(room - Math.floor(room / 2));
    return cell + " ".repeat(room);
  };
  const rule = (w, i) =>
    align[i] === "center" ? `:${"-".repeat(w - 2)}:` : align[i] === "right" ? `${"-".repeat(w - 1)}:` : align[i] === "left" ? `:${"-".repeat(w - 1)}` : "-".repeat(w);
  return rows.map((row) =>
    Array.isArray(row) ? `| ${row.map(pad).join(" | ")} |` : `| ${widths.map(rule).join(" | ")} |`,
  );
}

/**
 * An include copied into the pirate page, pointing at the same file: a
 * relative path to a file outside docs/cli gains a `../` for the page's place
 * under docs/pirate/. Paths inside docs/cli, which the pirate pages mirror,
 * and `@`-rooted ones stay as they are.
 */
function pirateInclude(line, page) {
  return line.replace(/(<!--\s*@include:\s*)(.*?)(\s*-->)/g, (whole, open, target, close) => {
    if (target.startsWith("@")) return whole;
    const [, path, meta] = /^(.*?)((?:#[\w-]+)?(?:\{\d*,\d*\})?)$/.exec(target);
    const from = posix.dirname(`cli/${page}`);
    const resolved = posix.normalize(posix.join(from, path));
    if (resolved.startsWith("cli/")) return whole;
    return `${open}${posix.relative(posix.join(PIRATE_DIR, from), resolved)}${meta}${close}`;
  });
}

/**
 * One English CLI page in the crew's words. `page` is its path under
 * docs/cli (`check.md`, `run/pre-commit.md`). Returns the pirate page's
 * front matter lines and body, and the English lines left untranslated.
 */
export function translatePage(english, page, words) {
  // Own properties only: an English line such as "constructor" is not a word the crew gave.
  const has = (map, key) => map != null && Object.hasOwn(map, key);
  const own = has(words.pages, page) ? words.pages[page] : {};
  const labels = Object.entries(words.labels ?? {});
  const untranslated = [];
  let missed = untranslated;
  const say = (text) => {
    if (has(own, text)) return own[text];
    if (has(words.lines, text)) return words.lines[text];
    for (const [from, to] of labels) if (text.startsWith(from)) return to + text.slice(from.length);
    if (hasProse(text)) missed.push(text);
    return text;
  };

  const lines = english.replace(/\r\n/g, "\n").split("\n");
  const front = [];
  const body = [];
  // Front matter a reader sees only in link previews: listed, not counted on the page.
  const frontUntranslated = [];
  let i = 0;
  if (lines[0] === "---") {
    const end = lines.indexOf("---", 1);
    missed = frontUntranslated;
    for (const line of lines.slice(1, end)) {
      // A command's page keeps the command as its title; only other titles have words to translate.
      const commandTitle = /^title: "hk(?: [^"]*)?"$/.test(line);
      front.push(/^(title|description):/.test(line) && !commandTitle ? say(line) : line);
    }
    missed = untranslated;
    i = end + 1;
  }

  const slugs = new Set();
  let fence = null;
  let comment = false;
  let table = [];
  const flushTable = () => {
    if (table.length) body.push(...formatTable(table));
    table = [];
  };
  for (; i < lines.length; i++) {
    const line = lines[i];
    if (comment) {
      body.push(line);
      if (line.includes("-->")) comment = false;
      continue;
    }
    if (fence) {
      body.push(line);
      const close = /^\s*(`{3,}|~{3,})\s*$/.exec(line);
      if (close && close[1][0] === fence[0] && close[1].length >= fence.length) fence = null;
      continue;
    }
    if (TABLE_RE.test(line)) {
      const cells = cellsOf(line);
      if (cells.some((cell) => (cell.match(/`/g) ?? []).length % 2)) {
        // An unescaped pipe inside code split a cell; the row is translated whole.
        flushTable();
        body.push(say(line));
      } else {
        table.push(RULE_RE.test(line) ? { align: cells.map(alignOf) } : cells.map((cell) => say(cell.trim())));
      }
      continue;
    }
    flushTable();
    const open = /^\s*(`{3,}|~{3,})/.exec(line);
    if (open) {
      fence = open[1];
      body.push(line);
      continue;
    }
    if (line.startsWith("<!-- @generated")) {
      body.push(
        `<!-- @generated by docs/.vitepress/pirate-cli.mjs from docs/cli/${page} and docs/pirate/cli.json; edit those, not this page. -->`,
      );
      continue;
    }
    if (line.trimStart().startsWith("<!--")) {
      body.push(pirateInclude(line, page));
      comment = !line.includes("-->");
      continue;
    }
    const heading = /^(#{1,6}) (.*?)(?: \{#([^}]+)\})?$/.exec(line);
    if (heading) {
      const [, hashes, text, custom] = heading;
      const base = custom ?? slugify(headingText(text));
      let id = base;
      for (let n = 1; slugs.has(id); n++) id = `${base}-${n}`;
      slugs.add(id);
      const said = say(`${hashes} ${text}`);
      // The words may bring the English anchor along (STYLE.md shows headings
      // that way); any other anchor would break every link to the section.
      const anchored = / \{#([^}]+)\}$/.exec(said);
      if (anchored && anchored[1] !== base) {
        throw new Error(
          `docs/pirate/cli.json: "${hashes} ${text}" gives the anchor {#${anchored[1]}}, but ${page} needs {#${base}}; give the words alone and the anchor is added`,
        );
      }
      const pirate = anchored ? said.slice(0, anchored.index) : said;
      // An h1 needs no anchor, and English words keep their own.
      body.push(hashes.length === 1 || (pirate === `${hashes} ${text}` && !custom) ? pirate : `${pirate} {#${id}}`);
      continue;
    }
    body.push(line.trim() ? say(line) : line);
  }
  flushTable();
  return { front, body: body.join("\n"), untranslated, frontUntranslated };
}

/** Every English CLI page, as its path under docs/cli, sorted. */
export function cliPages(docs = DOCS) {
  return globSync("cli/**/*.md", { cwd: docs })
    .map((p) => p.split("\\").join("/").slice("cli/".length))
    .sort();
}

/** The crew's words for the CLI reference. */
export function readWords(file = WORDS) {
  return existsSync(file) ? JSON.parse(readFileSync(file, "utf8")) : {};
}

/** The pirate page for docs/cli/<page>, as it is written to disk. */
export function pirateCliPage(page, words, docs = DOCS) {
  const english = readFileSync(join(docs, "cli", page), "utf8");
  const { front, body, untranslated, frontUntranslated } = translatePage(english, page, words);
  // Written from the English page as it is now, so it is never stale; the
  // lines still in English are counted for the page's notice.
  const extra = [`sourceHash: ${hashText(english)}`];
  if (untranslated.length) extra.push(`pirateUntranslated: ${untranslated.length}`);
  return { text: `---\n${[...front, ...extra].join("\n")}\n---\n${body}`, untranslated: [...frontUntranslated, ...untranslated] };
}

/** Rebuilds docs/pirate/cli: one page per English page, and none for pages that are gone. */
export function generate(words = readWords(), docs = DOCS) {
  const pages = cliPages(docs);
  const out = join(docs, PIRATE_DIR, "cli");
  const missing = [];
  for (const page of pages) {
    const { text, untranslated } = pirateCliPage(page, words, docs);
    mkdirSync(dirname(join(out, page)), { recursive: true });
    writeFileSync(join(out, page), text);
    for (const line of untranslated) missing.push({ page, line });
  }
  const keep = new Set(pages);
  for (const old of globSync("**/*.md", { cwd: out })) {
    if (!keep.has(old.split("\\").join("/"))) rmSync(join(out, old));
  }
  return { pages, missing };
}

/** The English lines of the CLI reference with no pirate words yet, without writing anything. */
export function missingWords(words = readWords(), docs = DOCS) {
  return cliPages(docs).flatMap((page) => pirateCliPage(page, words, docs).untranslated.map((line) => ({ page, line })));
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.includes("--missing")) {
    const missing = missingWords();
    for (const { page, line } of missing) console.log(`${page}: ${line}`);
    console.log(`${missing.length} English line${missing.length === 1 ? "" : "s"} of the CLI reference have no pirate words in docs/pirate/cli.json.`);
  } else {
    const { pages, missing } = generate();
    console.log(`wrote ${pages.length} pirate CLI pages; ${missing.length} line${missing.length === 1 ? "" : "s"} still in English (see --missing)`);
  }
}
