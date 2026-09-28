// Verify the built HTML references real, page-specific PNG previews, that only
// the homepage and the shanty's page offer their rendered videos (the
// showreel, the music video) as og:video, and that their players play the
// files deployed with them.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { socialCard } from "./social-images.mjs";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { resolve, join } from "node:path";

const root = resolve(process.argv[2] || ".vitepress/dist");
const metaTags = (html) =>
  [...html.matchAll(/<meta\b[^>]*>/g)].map(([tag]) =>
    Object.fromEntries(
      [...tag.matchAll(/([\w:-]+)=(?:"([^"]*)"|'([^']*)'|([^\s>]+))/g)].map(
        ([, name, quoted, single, bare]) => [name, quoted ?? single ?? bare],
      ),
    ),
  );
const meta = (html, key) => {
  const matches = metaTags(html).filter(
    (tag) => tag.property === key || tag.name === key,
  );
  assert.equal(matches.length, 1, `Expected one ${key} tag`);
  return matches[0].content;
};
// The version must change with the file, or players and previews keep a
// stale render.
const version = (file) =>
  createHash("sha256").update(file).digest("hex").slice(0, 12);
// The pages that offer a rendered video, each present only when its render
// ran before the build (`mise run docs:showreel`, `mise run docs:shanty`),
// with the poster its player shows until someone presses play.
const offers = [
  { page: "index.html", label: "The homepage", name: "the showreel", video: "showreel.mp4", poster: "showreel-poster.jpg" },
  { page: "shanty.html", label: "The shanty's page", name: "the music video", video: "bound-for-the-main.mp4", poster: "bound-for-the-main-poster.jpg" },
].map((o) => {
  const file = join(root, o.video);
  const bytes = existsSync(file) ? readFileSync(file) : null;
  const poster = bytes ? readFileSync(join(root, o.poster)) : null;
  if (poster) {
    assert.deepEqual(
      [...poster.subarray(0, 3)],
      [0xff, 0xd8, 0xff],
      `${o.poster} is not a JPEG`,
    );
  }
  return { ...o, bytes, posterBytes: poster };
});
const showreel = offers[0];
const video120File = join(root, "showreel-120.mp4");
const video120 = existsSync(video120File) ? readFileSync(video120File) : null;
const walk = (dir) =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? walk(join(dir, entry.name)) : [join(dir, entry.name)],
  );
let posts = 0;
const images = new Set();
for (const file of walk(root).filter((file) => file.endsWith(".html"))) {
  const html = readFileSync(file, "utf8");
  const home = file === join(root, "index.html");
  const offer = offers.find((o) => file === join(root, o.page));
  assert.equal(meta(html, "og:title"), meta(html, "twitter:title"));
  assert.equal(meta(html, "og:description"), meta(html, "twitter:description"));
  // Check the actual page title rather than only counting distinct image URLs.
  const decode = (value) =>
    value.replace(/&(amp|lt|gt|quot|apos|#\d+|#x[\da-f]+);/gi, (_, entity) => {
      if (entity.startsWith("#x"))
        return String.fromCodePoint(parseInt(entity.slice(2), 16));
      if (entity.startsWith("#"))
        return String.fromCodePoint(Number(entity.slice(1)));
      return { amp: "&", lt: "<", gt: ">", quot: '\"', apos: "'" }[entity];
    });
  const pageTitle = decode(meta(html, "og:title")).replace(/ \| hk$/, "");
  const heading = home ? "Fast git hooks and project linting" : pageTitle;
  const alt = meta(html, "og:image:alt");
  assert.ok(
    typeof alt === "string" && alt.trim(),
    `Empty image alt text: ${file}`,
  );
  assert.equal(
    decode(alt),
    heading + " — hk docs",
    `Wrong image alt text: ${file}`,
  );
  // X ignores og:video, so even the homepage keeps the large image card.
  assert.equal(meta(html, "twitter:card"), "summary_large_image");
  const expected = socialCard(heading);
  const image = meta(html, "og:image");
  assert.equal(meta(html, "twitter:image"), image);
  assert.equal(meta(html, "twitter:image:alt"), meta(html, "og:image:alt"));
  assert.equal(
    new URL(image).pathname,
    `/${expected.path}`,
    `Wrong page image: ${file}`,
  );
  assert.match(image, /^https:\/\//);
  assert.notEqual(new URL(image).pathname, "/og.png");
  const png = readFileSync(join(root, new URL(image).pathname));
  assert.deepEqual(png, expected.png, `Wrong image content: ${file}`);
  assert.deepEqual([...png.subarray(0, 8)], [137, 80, 78, 71, 13, 10, 26, 10]);
  assert.equal(png.readUInt32BE(16), 1200);
  assert.equal(png.readUInt32BE(20), 630);
  images.add(image);

  // meta() also asserts there is exactly one og:type.
  const videoTags = metaTags(html).filter((tag) =>
    tag.property?.startsWith("og:video"),
  );
  if (offer?.bytes) {
    const { video, poster, bytes, posterBytes } = offer;
    assert.equal(meta(html, "og:type"), "video.other", `Wrong og:type: ${file}`);
    const url = meta(html, "og:video");
    assert.equal(meta(html, "og:video:secure_url"), url);
    assert.equal(meta(html, "og:video:type"), "video/mp4");
    assert.equal(meta(html, "og:video:width"), "1920");
    assert.equal(meta(html, "og:video:height"), "1080");
    assert.match(url, /^https:\/\//);
    assert.equal(new URL(url).pathname, `/${video}`);
    assert.equal(
      new URL(url).searchParams.get("v"),
      version(bytes),
      `og:video is not the deployed ${video}`,
    );
    assert.equal(bytes.toString("latin1", 4, 8), "ftyp", `${video} is not an MP4`);
    // The player starts on the same (60 fps) file, with the poster rendered
    // with it, each versioned by its own bytes.
    const player = html.match(/<video\b[^>]*\ssrc="([^"]*)"/);
    assert.equal(
      player?.[1],
      `/${video}?v=${version(bytes)}`,
      `The player does not start on the deployed ${video}`,
    );
    const posterSrc = html.match(/<video\b[^>]*\sposter="([^"]*)"/);
    assert.equal(
      posterSrc?.[1],
      `/${poster}?v=${version(posterBytes)}`,
      "The player's poster is not the deployed render's",
    );
  } else {
    assert.equal(meta(html, "og:type"), "website", `Wrong og:type: ${file}`);
    assert.equal(videoTags.length, 0, `Unexpected og:video tags in ${file}`);
    if (offer) {
      assert.doesNotMatch(
        html,
        /<video\b/,
        `${offer.label} has a player but no ${offer.video}`,
      );
    }
  }
  posts++;
}
assert.ok(posts > 0, "No built pages found");
assert.ok(images.size > 1, "Pages should have distinct images");
if (showreel.bytes && video120) {
  // The player's script switches to the 120 fps file where it decodes well.
  assert.equal(
    video120.toString("latin1", 4, 8),
    "ftyp",
    "showreel-120.mp4 is not an MP4",
  );
  const src = `/showreel-120.mp4?v=${version(video120)}`;
  const scripts = walk(root).filter((file) => file.endsWith(".js"));
  assert.ok(
    scripts.some((file) => readFileSync(file, "utf8").includes(src)),
    `No script plays ${src}`,
  );
}
const rendered = offers.filter((o) => o.bytes).map((o) => o.name);
console.log(
  `Checked images and social metadata for ${posts} documentation pages${rendered.length ? `, with ${rendered.join(" and ")}` : ""}.`,
);
