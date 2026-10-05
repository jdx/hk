import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const configDir = dirname(fileURLToPath(import.meta.url));
const videoPath = resolve(configDir, "../public/showreel.mp4");
const video120Path = resolve(configDir, "../public/showreel-120.mp4");
const posterPath = resolve(configDir, "../public/showreel-poster.jpg");

export interface ShowreelFiles {
  /** Site-relative URL of the 60 fps MP4. */
  src: string;
  /**
   * The same reel at 120 fps, with its average bitrate in bits per second,
   * or null when this render has none.
   */
  video120: { src: string; bitrate: number } | null;
  /** Site-relative URL of its poster frame. */
  poster: string;
}

// Versioned so browsers and link previews that cached an earlier render fetch
// the new one.
const version = (file: Buffer) =>
  createHash("sha256").update(file).digest("hex").slice(0, 12);

/** The first box of this type between start and end, as [body, end]. */
function findBox(mp4: Buffer, type: string, start = 0, end = mp4.length) {
  for (let at = start; at + 8 <= end; ) {
    const size = mp4.readUInt32BE(at);
    if (size < 8) return null;
    if (mp4.toString("latin1", at + 4, at + 8) === type) return [at + 8, at + size];
    at += size;
  }
  return null;
}

/** An MP4's length in seconds, from its movie header. */
function mp4Seconds(mp4: Buffer): number | null {
  const moov = findBox(mp4, "moov");
  const mvhd = moov && findBox(mp4, "mvhd", moov[0], moov[1]);
  if (!mvhd) return null;
  // Version 1 widens the times to 64 bits.
  const at = mvhd[0];
  const [timescale, duration] =
    mp4[at] === 1
      ? [mp4.readUInt32BE(at + 20), Number(mp4.readBigUInt64BE(at + 24))]
      : [mp4.readUInt32BE(at + 12), mp4.readUInt32BE(at + 16)];
  return timescale && duration ? duration / timescale : null;
}

/**
 * The showreel rendered by `mise run docs:showreel`, which the docs deploy
 * runs before building, or null when this build has no render. The landing
 * page plays the 120 fps file where it decodes smoothly and the 60 fps file
 * elsewhere; the homepage offers the 60 fps file as og:video.
 */
export function showreelFiles(): ShowreelFiles | null {
  if (!existsSync(videoPath) || !existsSync(posterPath)) return null;
  const video = readFileSync(videoPath);
  const video120 = existsSync(video120Path) ? readFileSync(video120Path) : null;
  const seconds = video120 && mp4Seconds(video120);
  return {
    src: `/showreel.mp4?v=${version(video)}`,
    video120:
      video120 && seconds
        ? {
            src: `/showreel-120.mp4?v=${version(video120)}`,
            bitrate: Math.round((video120.length * 8) / seconds),
          }
        : null,
    // Versioned by its own bytes: a change to the poster frame alone must
    // reach browsers too.
    poster: `/showreel-poster.jpg?v=${version(readFileSync(posterPath))}`,
  };
}

export declare const data: ShowreelFiles | null;

export default {
  // Pick up a render made while the dev server is running.
  watch: [videoPath, video120Path, posterPath],
  load: showreelFiles,
};
