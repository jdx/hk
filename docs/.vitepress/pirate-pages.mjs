// The pirate pages: every English page under docs/ has a variant at
// docs/pirate/<same path>, written in the voice of the sea shanty (see
// docs/pirate/STYLE.md). This module finds the pairs, tells which variants are
// behind their English page, and checks that a variant carries everything a
// reader relies on: the same sections with the same anchors, the same code
// blocks byte for byte, the same links, inline code, containers, tables,
// images and components. config.mts uses it at build time; run it directly to
// see the state of the pages or to stamp a variant after writing it:
//
//   node .vitepress/pirate-pages.mjs status          # stale, missing and orphaned variants
//   node .vitepress/pirate-pages.mjs check [page…]   # compare variants with their English pages
//   node .vitepress/pirate-pages.mjs outline <page>  # what a variant of <page> must keep
//   node .vitepress/pirate-pages.mjs stamp <page…>   # record that a variant matches its English page
//
// Pages are named by their path under docs/, such as `hooks.md` or
// `cli/check.md`. A variant records the English page it was written from as a
// hash in its front matter (`sourceHash`). When the English page changes, the
// variant is stale: it still builds, shows a notice that links to the English
// page, and is listed by `status`, but the build never fails over it. A
// variant that is up to date must pass `check`.

import { createHash } from "node:crypto";
import { existsSync, globSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const DOCS = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const PIRATE_DIR = "pirate";
/** Files under docs/pirate/ that are not pages. */
export const PIRATE_NOT_PAGES = [`${PIRATE_DIR}/STYLE.md`];

/** English pages, relative to docs/, sorted. */
export function englishPages() {
  return globSync("**/*.md", {
    cwd: DOCS,
    exclude: (name) => ["node_modules", ".vitepress", "gen", "public", PIRATE_DIR].includes(name),
  })
    .map((p) => p.split("\\").join("/"))
    .sort();
}

/** Pages that have a variant under docs/pirate/, named by their English path, sorted. */
export function pirateVariants() {
  return globSync(`${PIRATE_DIR}/**/*.md`, { cwd: DOCS })
    .map((p) => p.split("\\").join("/"))
    .filter((p) => !PIRATE_NOT_PAGES.includes(p))
    .map((p) => p.slice(PIRATE_DIR.length + 1))
    .sort();
}

/**
 * The English page with the parts a release rewrites by itself taken out, so
 * a release does not make a variant stale: the version in every `package://`
 * URL and the CLI reference's `**Version:**` line, which
 * mise-tasks/update-version.sh updates in both pages alike.
 */
export function normalize(text) {
  return text
    .replace(/\r\n/g, "\n")
    .replace(/(package:\/\/github\.com\/jdx\/hk\/releases\/download\/)v[^/\s]+(\/hk@)[^#\s]+#/g, "$1vX$2X#")
    .replace(/^\*\*Version:\*\* \S+$/gm, "**Version:** X");
}

/** The hash a variant records for the English page it was written from. */
export function sourceHash(page) {
  return createHash("sha256")
    .update(normalize(readFileSync(join(DOCS, page), "utf8")))
    .digest("hex")
    .slice(0, 12);
}

const FRONT_MATTER_RE = /^---\r?\n([\s\S]*?)\r?\n---\r?\n/;
const HASH_LINE_RE = /^sourceHash:\s*["']?([0-9a-f]*)["']?\s*$/m;

/** The hash a page's front matter records, read as text (YAML would read 123456789012 as a number), or null. */
function hashIn(text) {
  const front = FRONT_MATTER_RE.exec(text);
  return (front && HASH_LINE_RE.exec(front[1])?.[1]) || null;
}

/** The hash a variant recorded, or null when it has none. */
export function recordedHash(page) {
  return hashIn(readFileSync(join(DOCS, PIRATE_DIR, page), "utf8"));
}

/** A page's text with `sourceHash: <hash>` in its front matter, added or replaced. */
export function stampText(text, hash) {
  const line = `sourceHash: ${hash}`;
  const front = FRONT_MATTER_RE.exec(text);
  if (!front) return `---\n${line}\n---\n\n${text}`;
  // Sliced, not replaced: a replacement string would read `$&` and the like in the page.
  const rest = text.slice(front[0].length);
  if (HASH_LINE_RE.test(front[1])) return front[0].replace(HASH_LINE_RE, () => line) + rest;
  return `---\n${front[1]}\n${line}\n---\n${rest}`;
}

/** Writes the English page's current hash into its variant's front matter. */
export function stamp(page) {
  const file = join(DOCS, PIRATE_DIR, page);
  writeFileSync(file, stampText(readFileSync(file, "utf8"), sourceHash(page)));
}

/**
 * Where the variants stand: `stale` lists variants written from an older
 * English page, `missing` English pages with no variant, and `orphans`
 * variants whose English page is gone.
 */
export function pirateStatus() {
  const english = englishPages();
  const variants = new Set(pirateVariants());
  const stale = [];
  for (const page of english) {
    if (variants.has(page) && recordedHash(page) !== sourceHash(page)) stale.push(page);
  }
  return {
    stale,
    missing: english.filter((page) => !variants.has(page)),
    orphans: [...variants].filter((page) => !existsSync(join(DOCS, page))),
  };
}

/** Whether a variant is up to date with its English page. */
export function isCurrent(page) {
  return existsSync(join(DOCS, PIRATE_DIR, page)) && recordedHash(page) === sourceHash(page);
}

/** Replaces `<!--@include: …-->` as VitePress does, so both pages are compared with their fragments. */
export function expandIncludes(text, file) {
  return text.replace(/<!--\s*@include:\s*(.*?)\s*-->/g, (whole, target) => {
    const path = target.startsWith("@") ? join(DOCS, target.slice(target[1] === "/" ? 2 : 1)) : join(dirname(file), target);
    if (!existsSync(path)) return whole;
    return expandIncludes(readFileSync(path, "utf8").replace(FRONT_MATTER_RE, ""), path);
  });
}

let renderer;
/**
 * VitePress's own Markdown parser, so anchors and containers are read as the
 * site builds them. Its front matter plugin works inside `render`, so this
 * renderer hands back the tokens instead of HTML.
 */
async function tokenize() {
  if (!renderer) {
    renderer = import("vitepress").then(async ({ createMarkdownRenderer }) => {
      const md = await createMarkdownRenderer(DOCS, { cache: false }, "/", { warn() {} });
      let captured = [];
      md.renderer.render = (tokens) => {
        captured = tokens;
        return "";
      };
      return (src, env) => {
        md.render(src, env);
        return captured;
      };
    });
  }
  return renderer;
}

/** What a reader relies on in a page, read from VitePress's tokens. */
export async function skeleton(file) {
  const parse = await tokenize();
  const source = readFileSync(file, "utf8");
  const env = { relativePath: relative(DOCS, file), path: file };
  const tokens = parse(expandIncludes(source, file), env);
  const out = {
    frontmatter: env.frontmatter ?? {},
    h1: 0,
    headings: [],
    code: [],
    links: new Set(),
    inlineCode: new Set(),
    containers: [],
    tables: [],
    images: new Set(),
    components: new Set(),
    interpolations: 0,
  };
  let table = null;
  const visit = (list) => {
    for (const token of list) {
      if (token.type === "heading_open") {
        const level = Number(token.tag.slice(1));
        if (level === 1) out.h1++;
        else out.headings.push(`h${level}#${token.attrGet("id")}`);
      } else if (token.type === "fence") {
        out.code.push(`${token.info.trim()}\n${normalize(token.content)}`);
      } else if (token.type === "link_open" && token.attrGet("class") !== "header-anchor") {
        // A heading's own permalink is not a link the page makes.
        out.links.add(token.attrGet("href"));
      } else if (token.type === "code_inline") {
        out.inlineCode.add(token.content);
      } else if (/^container_.+_open$/.test(token.type)) {
        out.containers.push(token.type.slice("container_".length, -"_open".length));
      } else if (token.type === "table_open") {
        table = { rows: 0, cols: 0 };
      } else if (token.type === "tr_open" && table) {
        table.rows++;
      } else if ((token.type === "th_open" || token.type === "td_open") && table && table.rows === 1) {
        table.cols++;
      } else if (token.type === "table_close" && table) {
        out.tables.push(`${table.rows}x${table.cols}`);
        table = null;
      } else if (token.type === "image") {
        out.images.add(token.attrGet("src"));
      } else if (token.type === "html_block" || token.type === "html_inline") {
        for (const [, name] of token.content.matchAll(/<([A-Z][A-Za-z0-9]*)\b/g)) out.components.add(name);
      } else if (token.type === "text") {
        out.interpolations += (token.content.match(/\{\{/g) ?? []).length;
      }
      if (token.children) visit(token.children);
    }
  };
  visit(tokens);
  return out;
}

/** Front matter a variant rewrites in its own words; every other key must match. */
const OWN_WORDS = new Set(["title", "description", "sourceHash"]);

const listed = (items) => [...items].map((item) => `    ${JSON.stringify(item)}`).join("\n");

/** The ways a page's variant falls short of it; empty when it keeps everything. */
export async function comparePage(page) {
  const pirateFile = join(DOCS, PIRATE_DIR, page);
  if (!existsSync(pirateFile)) return [`no variant at docs/${PIRATE_DIR}/${page}`];
  return compareFiles(join(DOCS, page), pirateFile);
}

/** The ways the variant in `pirateFile` falls short of the English page in `englishFile`. */
export async function compareFiles(englishFile, pirateFile) {
  const english = await skeleton(englishFile);
  const pirate = await skeleton(pirateFile);
  const problems = [];

  for (const key of new Set([...Object.keys(english.frontmatter), ...Object.keys(pirate.frontmatter)])) {
    if (OWN_WORDS.has(key)) continue;
    const a = JSON.stringify(english.frontmatter[key]);
    const b = JSON.stringify(pirate.frontmatter[key]);
    if (a !== b) problems.push(`front matter \`${key}\` is ${b ?? "missing"}; the English page has ${a ?? "none"}`);
  }
  for (const key of ["title", "description"]) {
    if (english.frontmatter[key] && !pirate.frontmatter[key]) problems.push(`front matter needs its own \`${key}\``);
  }
  if (!hashIn(readFileSync(pirateFile, "utf8"))) problems.push("front matter has no `sourceHash`; run `stamp` after writing the page");

  if (english.h1 !== pirate.h1) problems.push(`has ${pirate.h1} h1 headings; the English page has ${english.h1}`);
  if (english.headings.join() !== pirate.headings.join()) {
    problems.push(
      `sections differ. Each heading needs the English heading's level and anchor, in order, written as \`## Pirate words {#anchor}\`.\n  English:\n${listed(english.headings)}\n  Pirate:\n${listed(pirate.headings)}`,
    );
  }
  english.code.forEach((block, i) => {
    if (pirate.code[i] !== block) {
      problems.push(`code block ${i + 1} differs; copy it from the English page unchanged:\n${block.replace(/^/gm, "    | ")}`);
    }
  });
  if (pirate.code.length > english.code.length) problems.push(`has ${pirate.code.length - english.code.length} code blocks the English page does not`);

  const missingLinks = [...english.links].filter((link) => !pirate.links.has(link));
  const extraLinks = [...pirate.links].filter((link) => !english.links.has(link));
  if (missingLinks.length) problems.push(`drops links the English page has:\n${listed(missingLinks)}`);
  if (extraLinks.length) problems.push(`adds links the English page does not have:\n${listed(extraLinks)}`);

  const missingCode = [...english.inlineCode].filter((code) => !pirate.inlineCode.has(code));
  if (missingCode.length) problems.push(`drops inline code the English page has (keep each, unchanged):\n${listed(missingCode)}`);

  if (english.containers.join() !== pirate.containers.join()) {
    problems.push(`containers differ: English ${JSON.stringify(english.containers)}, pirate ${JSON.stringify(pirate.containers)}`);
  }
  if (english.tables.join() !== pirate.tables.join()) {
    problems.push(`tables differ (rows x columns): English ${JSON.stringify(english.tables)}, pirate ${JSON.stringify(pirate.tables)}`);
  }
  for (const [name, a, b] of [
    ["images", english.images, pirate.images],
    ["components", english.components, pirate.components],
  ]) {
    if ([...a].sort().join() !== [...b].sort().join()) problems.push(`${name} differ: English ${JSON.stringify([...a])}, pirate ${JSON.stringify([...b])}`);
  }
  if (pirate.interpolations > english.interpolations) {
    problems.push("has `{{` in its prose, which Vue reads as an expression; reword it or put it in inline code");
  }
  return problems;
}

async function main(args) {
  const [command = "status", ...pages] = args;
  if (command === "status") {
    const { stale, missing, orphans } = pirateStatus();
    const total = englishPages().length;
    console.log(
      `${total - missing.length} of ${total} English pages have a pirate variant; ${stale.length} ${stale.length === 1 ? "is" : "are"} behind their English page.`,
    );
    for (const [label, list] of [
      ["behind their English page (rewrite, then `stamp`)", stale],
      ["with no pirate variant", missing],
      ["variants of English pages that are gone", orphans],
    ]) {
      if (list.length) console.log(`\n${list.length} ${label}:\n${list.map((p) => `  ${p}`).join("\n")}`);
    }
    return 0;
  }
  if (command === "stamp") {
    if (!pages.length) throw new Error("name the pages to stamp, such as `stamp hooks.md`");
    for (const page of pages) {
      stamp(page);
      console.log(`stamped ${PIRATE_DIR}/${page} (${sourceHash(page)})`);
    }
    return 0;
  }
  if (command === "outline") {
    for (const page of pages) {
      const s = await skeleton(join(DOCS, page));
      console.log(
        [
          `# ${page} (sourceHash ${sourceHash(page)})`,
          `front matter keys: ${Object.keys(s.frontmatter).join(", ") || "none"}`,
          `h1 headings: ${s.h1}`,
          `sections, in order (level#anchor):\n${listed(s.headings)}`,
          `code blocks: ${s.code.length} (copy each unchanged)`,
          `links:\n${listed(s.links)}`,
          `inline code:\n${listed(s.inlineCode)}`,
          `containers: ${JSON.stringify(s.containers)}`,
          `tables (rows x columns): ${JSON.stringify(s.tables)}`,
          `images: ${JSON.stringify([...s.images])}`,
          `components: ${JSON.stringify([...s.components])}`,
        ].join("\n"),
      );
    }
    return 0;
  }
  if (command === "check") {
    const targets = pages.length ? pages : pirateVariants().filter((page) => existsSync(join(DOCS, page)));
    let failed = 0;
    let stale = 0;
    for (const page of targets) {
      if (!existsSync(join(DOCS, PIRATE_DIR, page))) {
        console.log(`✗ ${page}: no variant at docs/${PIRATE_DIR}/${page}`);
        failed++;
        continue;
      }
      const current = isCurrent(page);
      // Named pages are always checked; a full check skips stale ones, which
      // show their notice until someone rewrites them.
      if (!current && !pages.length) {
        stale++;
        continue;
      }
      const problems = await comparePage(page);
      if (!current) problems.unshift("is behind its English page: rewrite what changed, then `stamp` it");
      if (problems.length) {
        failed++;
        console.log(`✗ ${page}\n${problems.map((p) => `  - ${p}`).join("\n")}`);
      } else {
        console.log(`✓ ${page}`);
      }
    }
    console.log(`\n${targets.length - failed - stale} passed, ${failed} failed${stale ? `, ${stale} stale and not checked (see \`status\`)` : ""}.`);
    return failed ? 1 : 0;
  }
  throw new Error(`unknown command ${command}; use status, check, outline or stamp`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).then(
    (code) => process.exit(code),
    (error) => {
      console.error(error.message);
      process.exit(2);
    },
  );
}
